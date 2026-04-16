//! Research API helpers — company profiles, earnings, transcripts, IPOs, peers,
//! press releases, social sentiment, commodities futures quotes.
//!
//! Sources:
//! - Finnhub free tier: /stock/profile2, /stock/peers, /stock/earnings,
//!   /stock/social-sentiment, /press-releases, /calendar/ipo
//! - FMP free tier: /earning_call_transcript, /historical/earning_calendar
//! - Yahoo Finance: /v7/finance/quote (commodities, cross-asset quotes)
//!
//! All functions take an existing reqwest::Client so callers control the HTTP stack
//! (rate limiting, user-agent, timeouts).
//!
//! Research results are cached in SQLite so MT5/Darwinex symbols only need to hit
//! the APIs once per scrape cycle — the DES/PEERS/EARNINGS/PRESS/SENTIMENT/
//! TRANSCRIPTS windows read from cache first and fall back to live fetch.

use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};

// ── Data Types ─────────────────────────────────────────────────────────────

/// Unified company profile — DES command backing data.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CompanyProfile {
    pub symbol: String,
    pub name: String,
    pub exchange: String,
    pub country: String,
    pub currency: String,
    pub industry: String,
    pub sector: String,
    pub website: String,
    pub logo: String,
    pub phone: String,
    pub ipo_date: String,
    pub market_cap: f64,            // in USD millions (Finnhub native unit)
    pub shares_outstanding: f64,    // in millions
}

/// One row in the earnings history (actual vs estimate EPS).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarningRow {
    pub period: String,    // YYYY-MM-DD
    pub actual: Option<f64>,
    pub estimate: Option<f64>,
    pub surprise: Option<f64>,
    pub surprise_pct: Option<f64>,
    pub quarter: Option<i32>,
    pub year: Option<i32>,
}

/// IPO calendar row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IpoEvent {
    pub date: String,
    pub symbol: String,
    pub name: String,
    pub exchange: String,
    pub price_range: String,
    pub shares: i64,
    pub total_value: f64,
    pub status: String,
}

/// Earnings call transcript list entry (metadata only).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TranscriptMeta {
    pub symbol: String,
    pub quarter: i32,
    pub year: i32,
    pub date: String,
}

/// Full transcript content.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Transcript {
    pub symbol: String,
    pub quarter: i32,
    pub year: i32,
    pub date: String,
    pub content: String,
}

/// Social sentiment snapshot (Reddit + Twitter combined from Finnhub).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SocialSentimentRow {
    pub source: String,      // "reddit" | "twitter"
    pub at_time: String,
    pub mention: i64,
    pub positive_mention: i64,
    pub negative_mention: i64,
    pub positive_score: f64,
    pub negative_score: f64,
    pub score: f64,
}

/// Press release item.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PressRelease {
    pub symbol: String,
    pub datetime: String,
    pub headline: String,
    pub description: String,
    pub url: String,
}

/// Commodity futures quote row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommodityQuote {
    pub symbol: String,      // e.g. "GC=F"
    pub display: String,     // e.g. "Gold"
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
}

// ── ADR-109 Godel Parity Round 2 types ─────────────────────────────────────

/// DVD — single historical dividend payment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DividendRecord {
    pub ex_date: String,            // YYYY-MM-DD
    pub pay_date: String,
    pub record_date: String,
    pub declaration_date: String,
    pub amount: f64,                // cash per share
    pub adjusted_amount: f64,       // split-adjusted
    pub label: String,              // e.g. "Regular Cash"
}

/// EEB — one forward earnings estimate row (one fiscal period).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarningsEstimate {
    pub date: String,               // period end YYYY-MM-DD
    pub eps_avg: f64,
    pub eps_high: f64,
    pub eps_low: f64,
    pub revenue_avg: f64,
    pub revenue_high: f64,
    pub revenue_low: f64,
    pub num_analysts_eps: i32,
    pub num_analysts_rev: i32,
}

/// UPDG — one analyst rating change (upgrade/downgrade/initiation).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RatingChange {
    pub date: String,               // YYYY-MM-DD
    pub symbol: String,
    pub company: String,
    pub firm: String,               // publisher / analyst house
    pub action: String,             // "upgrade" | "downgrade" | "initiation" | "maintain"
    pub from_grade: String,
    pub to_grade: String,
    pub price_target: f64,
}

/// GY — US Treasury yield curve snapshot row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TreasuryYield {
    pub tenor: String,              // "13W" | "5Y" | "10Y" | "30Y"
    pub ticker: String,              // Yahoo ticker ^IRX etc
    pub yield_pct: f64,
    pub change: f64,
    pub change_pct: f64,
}

/// Hardcoded Treasury yield ladder — Yahoo tickers only (free, no key).
pub const TREASURY_TENORS: &[(&str, &str)] = &[
    ("^IRX", "13W"),
    ("^FVX", "5Y"),
    ("^TNX", "10Y"),
    ("^TYX", "30Y"),
];

// ── ADR-110 Godel Parity Round 3 types ─────────────────────────────────────

/// FA — one fiscal period of an Income Statement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IncomeStatement {
    pub date: String,                  // period end YYYY-MM-DD
    pub period: String,                // "FY" | "Q1" | "Q2" | "Q3" | "Q4"
    pub revenue: f64,
    pub cost_of_revenue: f64,
    pub gross_profit: f64,
    pub research_and_development: f64,
    pub selling_general_admin: f64,
    pub operating_expenses: f64,
    pub operating_income: f64,
    pub interest_expense: f64,
    pub ebitda: f64,
    pub income_before_tax: f64,
    pub income_tax_expense: f64,
    pub net_income: f64,
    pub eps: f64,
    pub eps_diluted: f64,
    pub weighted_shares_out: f64,
}

/// FA — one fiscal period of a Balance Sheet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BalanceSheet {
    pub date: String,
    pub period: String,
    pub cash_and_equiv: f64,
    pub short_term_investments: f64,
    pub net_receivables: f64,
    pub inventory: f64,
    pub total_current_assets: f64,
    pub property_plant_equipment: f64,
    pub goodwill: f64,
    pub intangible_assets: f64,
    pub long_term_investments: f64,
    pub total_non_current_assets: f64,
    pub total_assets: f64,
    pub accounts_payable: f64,
    pub short_term_debt: f64,
    pub total_current_liabilities: f64,
    pub long_term_debt: f64,
    pub total_non_current_liabilities: f64,
    pub total_liabilities: f64,
    pub common_stock: f64,
    pub retained_earnings: f64,
    pub total_equity: f64,
    pub total_debt: f64,
    pub net_debt: f64,
}

/// FA — one fiscal period of a Cash Flow Statement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CashFlowStatement {
    pub date: String,
    pub period: String,
    pub net_income: f64,
    pub depreciation_amortization: f64,
    pub stock_based_comp: f64,
    pub change_working_capital: f64,
    pub cash_from_operations: f64,
    pub capex: f64,
    pub acquisitions: f64,
    pub investments_purchases: f64,
    pub cash_from_investing: f64,
    pub debt_repayment: f64,
    pub dividends_paid: f64,
    pub stock_repurchases: f64,
    pub cash_from_financing: f64,
    pub net_change_cash: f64,
    pub free_cash_flow: f64,
}

/// FA — combined bundle of all 3 statements × (annual/quarterly) for a symbol.
/// Serialized as a single JSON blob in research_financials so one SQL row covers the whole view.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FinancialStatements {
    pub income_annual: Vec<IncomeStatement>,
    pub income_quarterly: Vec<IncomeStatement>,
    pub balance_annual: Vec<BalanceSheet>,
    pub balance_quarterly: Vec<BalanceSheet>,
    pub cashflow_annual: Vec<CashFlowStatement>,
    pub cashflow_quarterly: Vec<CashFlowStatement>,
}

/// MGMT — one company officer / executive.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Executive {
    pub name: String,
    pub position: String,
    pub age: i32,
    pub sex: String,
    pub since: String,      // year joined role (string to handle Finnhub "N/A")
    pub compensation: f64,  // USD total comp for the year
    pub year: i32,          // comp reporting year
}

/// COT — one CFTC Commitment of Traders weekly row (legacy futures).
/// Global snapshot, not per-symbol. Not persisted (weekly refresh is fast, staleness meaningless).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CotReport {
    pub market_name: String,       // e.g. "GOLD - COMMODITY EXCHANGE INC."
    pub market_code: String,       // CFTC contract market code
    pub report_date: String,       // YYYY-MM-DD
    pub open_interest: f64,
    // Non-commercial (large speculators)
    pub noncomm_long: f64,
    pub noncomm_short: f64,
    pub noncomm_spreads: f64,
    // Commercial (producers / hedgers)
    pub comm_long: f64,
    pub comm_short: f64,
    // Non-reportable (small traders)
    pub nonrept_long: f64,
    pub nonrept_short: f64,
    // Derived: non-commercial net + week-over-week change
    pub noncomm_net: f64,
    pub noncomm_net_change: f64,
}

// ── ADR-111 Godel Parity Round 4 types ─────────────────────────────────────

/// SPLT — one historical stock split event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StockSplit {
    pub date: String,              // YYYY-MM-DD
    pub label: String,             // "2:1" | "3:2" etc.
    pub numerator: f64,             // new shares
    pub denominator: f64,           // old shares
}

/// ETF — one constituent holding of an exchange-traded fund.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EtfHolding {
    pub symbol: String,             // held company ticker
    pub name: String,               // held company name
    pub weight_pct: f64,            // % of ETF AUM
    pub shares: f64,
    pub market_value: f64,
    pub updated: String,            // as-of date
}

/// ANR — analyst recommendation bucket trend for a single period.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalystRecommendation {
    pub period: String,             // YYYY-MM-DD (end of reporting month)
    pub strong_buy: i32,
    pub buy: i32,
    pub hold: i32,
    pub sell: i32,
    pub strong_sell: i32,
}

/// ANR — consensus price target snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriceTarget {
    pub symbol: String,
    pub target_high: f64,
    pub target_low: f64,
    pub target_mean: f64,
    pub target_median: f64,
    pub last_updated: String,       // YYYY-MM-DD
    pub num_analysts: i32,
}

/// ESG — environmental / social / governance risk score.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EsgScore {
    pub symbol: String,
    pub environmental_score: f64,
    pub social_score: f64,
    pub governance_score: f64,
    pub esg_score: f64,             // weighted composite
    pub year: i32,
}

/// MEMB — one member company of an equity index.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexMember {
    pub index: String,              // "SP500" | "NDX" | "DJIA"
    pub symbol: String,
    pub name: String,
    pub sector: String,
    pub sub_sector: String,
    pub headquarters: String,
    pub date_added: String,         // YYYY-MM-DD when admitted to index
}

// ── ADR-112 Godel Parity Round 5 ─────────────────────────────────────────

/// INS — one insider trade filing (Form 4 row).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsiderTrade {
    pub filing_date: String,        // YYYY-MM-DD when filed with SEC
    pub transaction_date: String,   // YYYY-MM-DD of the trade itself
    pub reporting_name: String,     // insider who filed
    pub transaction_type: String,   // "P-Purchase", "S-Sale", "M-Exempt", "A-Award", etc.
    pub acquisition_disposition: String, // "A" (acquired) or "D" (disposed)
    pub shares: f64,                // securitiesTransacted
    pub price: f64,                 // per-share price
    pub value_usd: f64,             // shares * price (derived)
    pub shares_owned_after: f64,    // securitiesOwned post-trade
    pub link: String,               // SEC EDGAR filing URL
}

/// HDS — one institutional holder row (13F-derived).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstitutionalHolder {
    pub holder: String,             // fund / manager name
    pub shares: f64,                // shares held
    pub date_reported: String,      // 13F as-of date
    pub change: f64,                // delta shares vs prior quarter
}

/// FLOAT — shares float breakdown snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SharesFloat {
    pub symbol: String,
    pub date: String,               // YYYY-MM-DD snapshot date
    pub free_float_pct: f64,        // % of outstanding that is free-float
    pub float_shares: f64,          // absolute free float
    pub outstanding_shares: f64,    // total shares outstanding
    pub source: String,             // data provider
}

/// HP — one OHLCV daily bar for historical price table.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoricalPriceRow {
    pub date: String,               // YYYY-MM-DD
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub adj_close: f64,
    pub volume: f64,
    pub change: f64,                // close - open (USD)
    pub change_pct: f64,            // % change (close vs prior close)
}

/// EPS — one earnings surprise row (actual vs estimate).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarningsSurprise {
    pub date: String,               // report date YYYY-MM-DD
    pub symbol: String,
    pub eps_actual: f64,
    pub eps_estimate: f64,
    pub surprise: f64,              // actual - estimate
    pub surprise_pct: f64,          // (actual - estimate) / |estimate| * 100
}

// ── ADR-113 Godel Parity Round 6 ─────────────────────────────────────────

/// WEI — one global equity index quote row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldIndex {
    pub ticker: String,             // Yahoo ticker e.g. "^GSPC"
    pub display: String,            // human name "S&P 500"
    pub region: String,             // "Americas" | "Europe" | "Asia-Pacific"
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
}

/// MOV — one row inside a market movers list (gainers/losers/actives).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketMover {
    pub symbol: String,
    pub name: String,
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
    pub volume: f64,
}

/// MOV — bundle of three mover groups: top gainers, top losers, most active.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarketMovers {
    pub gainers: Vec<MarketMover>,
    pub losers: Vec<MarketMover>,
    pub actives: Vec<MarketMover>,
}

/// INDU — one sector performance row (intraday % change of a GICS sector ETF).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SectorPerformance {
    pub sector: String,             // "Technology", "Energy", …
    pub change_pct: f64,            // % change (absolute, e.g. 1.23 = +1.23 %)
}

/// WACC — derived weighted-average cost of capital snapshot.
/// Built from FMP profile/key-metrics + cached GY 10Y yield (risk-free rate)
/// using the standard CAPM cost-of-equity and after-tax cost-of-debt formulas.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WaccSnapshot {
    pub symbol: String,
    pub as_of: String,              // YYYY-MM-DD snapshot date (usually "today")
    pub beta: f64,                  // equity beta from FMP profile
    pub risk_free_pct: f64,         // 10Y Treasury yield %
    pub equity_risk_premium_pct: f64, // assumed ERP (5.0 % default)
    pub cost_of_equity_pct: f64,    // Rf + β × ERP
    pub pre_tax_cost_of_debt_pct: f64, // interest expense / total debt × 100
    pub tax_rate_pct: f64,          // effective tax rate %
    pub after_tax_cost_of_debt_pct: f64, // pre-tax × (1 - tax_rate)
    pub market_cap: f64,            // equity market value (USD)
    pub total_debt: f64,            // book debt (USD, proxy for market debt)
    pub equity_weight: f64,         // E / (E+D)  (0..1)
    pub debt_weight: f64,           // D / (E+D)  (0..1)
    pub wacc_pct: f64,              // we * Re + wd * Rd_after_tax
}

/// Hardcoded global equity index universe for the WEI dashboard.
/// Yahoo index tickers — all free via /v7/finance/quote.
pub const WORLD_INDICES_UNIVERSE: &[(&str, &str, &str)] = &[
    // Americas
    ("^GSPC",  "S&P 500",              "Americas"),
    ("^DJI",   "Dow Jones",            "Americas"),
    ("^IXIC",  "Nasdaq Composite",     "Americas"),
    ("^RUT",   "Russell 2000",         "Americas"),
    ("^GSPTSE","S&P/TSX Composite",    "Americas"),
    ("^BVSP",  "Ibovespa",             "Americas"),
    ("^MXX",   "IPC Mexico",           "Americas"),
    // Europe / Middle East / Africa
    ("^FTSE",  "FTSE 100",             "EMEA"),
    ("^GDAXI", "DAX",                  "EMEA"),
    ("^FCHI",  "CAC 40",               "EMEA"),
    ("^STOXX50E","Euro Stoxx 50",      "EMEA"),
    ("^IBEX",  "IBEX 35",              "EMEA"),
    ("FTSEMIB.MI","FTSE MIB",          "EMEA"),
    ("^AEX",   "AEX",                  "EMEA"),
    ("^SSMI",  "SMI",                  "EMEA"),
    // Asia-Pacific
    ("^N225",  "Nikkei 225",           "Asia-Pacific"),
    ("^HSI",   "Hang Seng",            "Asia-Pacific"),
    ("000001.SS","Shanghai Composite", "Asia-Pacific"),
    ("^AXJO",  "S&P/ASX 200",          "Asia-Pacific"),
    ("^KS11",  "KOSPI",                "Asia-Pacific"),
    ("^TWII",  "TSEC (Taiwan)",        "Asia-Pacific"),
    ("^BSESN", "BSE SENSEX",           "Asia-Pacific"),
];

/// Default equity risk premium used in the WACC CAPM calc (Damodaran-style).
pub const DEFAULT_EQUITY_RISK_PREMIUM_PCT: f64 = 5.0;

// ── ADR-114 Godel Parity Round 7 ─────────────────────────────────────────
// WCR / BETA / DDM / RV / FIGI surfaces.

/// WCR — single currency-cross row for the World Currency Rates dashboard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrencyRate {
    pub ticker: String,       // Yahoo ticker, e.g. "EURUSD=X"
    pub display: String,      // "EUR/USD"
    pub base: String,         // "EUR"
    pub quote: String,        // "USD"
    pub region: String,       // "Majors" / "Crosses" / "EM"
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
}

/// BETA — one rolling-window beta observation (e.g. 1Y/3Y/5Y).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BetaWindow {
    pub window_label: String,  // "1Y" / "3Y" / "5Y"
    pub window_days: usize,    // trading-day window (252 / 756 / 1260)
    pub beta: f64,             // cov(r_s, r_m) / var(r_m)
    pub alpha_pct: f64,        // annualized intercept
    pub r_squared: f64,
    pub n_observations: usize,
    pub correlation: f64,
}

/// BETA — per-symbol beta history snapshot (vs SPY) cached in SQLite.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BetaSnapshot {
    pub symbol: String,
    pub market_ticker: String, // "SPY"
    pub as_of: String,         // YYYY-MM-DD
    pub windows: Vec<BetaWindow>,
    pub note: String,          // any caveats (insufficient data, etc.)
}

/// DDM — Gordon Growth (two-stage optional) dividend discount model snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DdmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub annual_dividend: f64,       // trailing 4-quarter dividend $
    pub implied_growth_pct: f64,    // inferred from historical dividend CAGR
    pub required_return_pct: f64,   // from WACC or cost of equity
    pub growth_source: String,      // "dividend CAGR 5Y" etc.
    pub return_source: String,      // "WACC 10.25%" etc.
    pub implied_price: f64,         // D1 / (r - g) — 0.0 when r <= g
    pub method: String,             // "Gordon Growth"
    pub note: String,               // any caveats
}

/// RV — one metric row in the relative-valuation peer matrix.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RvMetricRow {
    pub metric: String,       // "P/E", "P/B", "EV/EBITDA", etc.
    pub value: f64,           // subject symbol's value
    pub peer_median: f64,
    pub peer_low: f64,
    pub peer_high: f64,
    pub z_score: f64,         // (value - mean) / stdev
    pub percentile: f64,      // 0..100 within peer set
}

/// RV — full relative-valuation snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelativeValuation {
    pub symbol: String,
    pub sector: String,
    pub as_of: String,
    pub peer_count: usize,
    pub rows: Vec<RvMetricRow>,
}

/// FIGI — one identifier mapping returned by OpenFIGI.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FigiIdentifier {
    pub figi: String,              // share-class / instrument FIGI
    pub name: String,
    pub ticker: String,
    pub exch_code: String,
    pub composite_figi: String,
    pub share_class_figi: String,
    pub security_type: String,
    pub security_type_2: String,
    pub market_sector: String,
    pub security_description: String,
}

/// FIGI — wrapper stored per-symbol in SQLite (list because a ticker can map
/// to multiple share classes / exchanges).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FigiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub identifiers: Vec<FigiIdentifier>,
}

/// Hardcoded FX-majors universe for the WCR dashboard.
/// Yahoo FX pair tickers — all free via /v7/finance/quote.
pub const FX_MAJORS_UNIVERSE: &[(&str, &str, &str, &str, &str)] = &[
    // ticker, display, base, quote, region
    ("EURUSD=X", "EUR/USD", "EUR", "USD", "Majors"),
    ("GBPUSD=X", "GBP/USD", "GBP", "USD", "Majors"),
    ("USDJPY=X", "USD/JPY", "USD", "JPY", "Majors"),
    ("USDCHF=X", "USD/CHF", "USD", "CHF", "Majors"),
    ("AUDUSD=X", "AUD/USD", "AUD", "USD", "Majors"),
    ("USDCAD=X", "USD/CAD", "USD", "CAD", "Majors"),
    ("NZDUSD=X", "NZD/USD", "NZD", "USD", "Majors"),
    // Common crosses
    ("EURJPY=X", "EUR/JPY", "EUR", "JPY", "Crosses"),
    ("EURGBP=X", "EUR/GBP", "EUR", "GBP", "Crosses"),
    ("EURCHF=X", "EUR/CHF", "EUR", "CHF", "Crosses"),
    ("GBPJPY=X", "GBP/JPY", "GBP", "JPY", "Crosses"),
    ("AUDJPY=X", "AUD/JPY", "AUD", "JPY", "Crosses"),
    ("CHFJPY=X", "CHF/JPY", "CHF", "JPY", "Crosses"),
    // Emerging-market USD pairs
    ("USDMXN=X", "USD/MXN", "USD", "MXN", "EM"),
    ("USDZAR=X", "USD/ZAR", "USD", "ZAR", "EM"),
    ("USDTRY=X", "USD/TRY", "USD", "TRY", "EM"),
    ("USDBRL=X", "USD/BRL", "USD", "BRL", "EM"),
    ("USDINR=X", "USD/INR", "USD", "INR", "EM"),
    ("USDCNY=X", "USD/CNY", "USD", "CNY", "EM"),
];

/// Hardcoded commodity-futures universe for the GLCO dashboard.
/// Yahoo continuous-futures tickers, which are free via /v7/finance/quote.
pub const COMMODITIES_UNIVERSE: &[(&str, &str, &str)] = &[
    // Precious metals
    ("GC=F", "Gold",        "Metals"),
    ("SI=F", "Silver",      "Metals"),
    ("PL=F", "Platinum",    "Metals"),
    ("PA=F", "Palladium",   "Metals"),
    ("HG=F", "Copper",      "Metals"),
    // Energy
    ("CL=F", "WTI Crude",   "Energy"),
    ("BZ=F", "Brent Crude", "Energy"),
    ("NG=F", "Natural Gas", "Energy"),
    ("HO=F", "Heating Oil", "Energy"),
    ("RB=F", "Gasoline",    "Energy"),
    // Grains
    ("ZC=F", "Corn",        "Grains"),
    ("ZS=F", "Soybeans",    "Grains"),
    ("ZW=F", "Wheat",       "Grains"),
    ("ZO=F", "Oats",        "Grains"),
    ("ZR=F", "Rice",        "Grains"),
    // Softs
    ("KC=F", "Coffee",      "Softs"),
    ("SB=F", "Sugar",       "Softs"),
    ("CT=F", "Cotton",      "Softs"),
    ("CC=F", "Cocoa",       "Softs"),
    ("OJ=F", "Orange Juice","Softs"),
    // Livestock
    ("LE=F", "Live Cattle", "Livestock"),
    ("HE=F", "Lean Hogs",   "Livestock"),
    ("GF=F", "Feeder Cattle","Livestock"),
];

// ── ADR-115 Godel Parity Round 8 ─────────────────────────────────────────
// HRA / DCF / SVM / OMON / IVOL surfaces.

/// HRA — one rolling-period return row (e.g. 1M, 3M, 1Y, YTD).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HraWindow {
    pub label: String,            // "1D" / "5D" / "1M" / "3M" / "6M" / "YTD" / "1Y" / "3Y" / "5Y" / "ITD"
    pub trading_days: usize,      // 0 for YTD/ITD which span by date
    pub return_pct: f64,          // simple return (pct)
    pub cagr_pct: f64,            // annualized when trading_days > 252
    pub n_observations: usize,
}

/// HRA — historical return + risk snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HraSnapshot {
    pub symbol: String,
    pub as_of: String,            // YYYY-MM-DD
    pub last_close: f64,
    pub windows: Vec<HraWindow>,
    pub max_drawdown_pct: f64,    // ITD, negative number
    pub drawdown_peak_date: String,
    pub drawdown_trough_date: String,
    pub volatility_annual_pct: f64, // stdev of daily log-returns × sqrt(252) × 100
    pub sharpe_ratio: f64,        // (mean daily return - rf) / stdev, annualized
    pub sortino_ratio: f64,       // same but downside deviation denominator
    pub calmar_ratio: f64,        // CAGR / |max_drawdown|
    pub risk_free_pct: f64,       // used in Sharpe/Sortino
    pub note: String,
}

/// DCF — one projection year in the explicit forecast period.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DcfYear {
    pub year: i32,                // calendar year or offset
    pub revenue: f64,
    pub ebit: f64,
    pub nopat: f64,               // NOPAT = EBIT × (1 - t)
    pub fcff: f64,                // free cash flow to firm
    pub discount_factor: f64,
    pub pv_fcff: f64,             // fcff × discount_factor
}

/// DCF — Discounted Cash Flow fair value snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DcfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub method: String,           // "DCF on FCFF"
    pub base_revenue: f64,
    pub base_fcff: f64,
    pub growth_pct: f64,          // explicit-period revenue growth
    pub terminal_growth_pct: f64, // Gordon growth in perpetuity
    pub wacc_pct: f64,            // discount rate
    pub tax_rate_pct: f64,
    pub fcff_margin_pct: f64,     // fcff / revenue applied to projections
    pub projection_years: usize,
    pub years: Vec<DcfYear>,
    pub pv_sum: f64,              // Σ pv of explicit FCFF
    pub terminal_value: f64,      // TV at end of explicit period
    pub pv_terminal: f64,         // TV × final discount factor
    pub enterprise_value: f64,    // pv_sum + pv_terminal
    pub total_debt: f64,
    pub cash_and_equivalents: f64,
    pub equity_value: f64,        // EV - debt + cash
    pub shares_outstanding: f64,
    pub implied_price: f64,       // equity_value / shares
    pub note: String,
}

/// SVM — one row in the multi-model fair-value triangulation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SvmModelRow {
    pub model: String,            // "WACC cost of equity" / "DDM Gordon Growth" / "DCF FCFF" / "RV P/E median" / "RV EV/EBITDA median"
    pub implied_price: f64,       // 0.0 if N/A
    pub current_price: f64,
    pub upside_pct: f64,          // (implied / current - 1) × 100
    pub confidence: String,       // "high" / "medium" / "low" / "n/a"
    pub source: String,           // short lineage
}

/// SVM — Stock Valuation Model summary for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SvmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub current_price: f64,
    pub rows: Vec<SvmModelRow>,
    pub fair_low: f64,            // min of non-zero implied prices
    pub fair_high: f64,           // max of non-zero implied prices
    pub fair_mid: f64,            // simple mean of non-zero implied prices
    pub upside_mid_pct: f64,      // (fair_mid / current - 1) × 100
    pub note: String,
}

/// OMON — one options contract row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptionContract {
    pub contract_symbol: String,  // e.g. "AAPL240419C00150000"
    pub option_type: String,      // "CALL" / "PUT"
    pub strike: f64,
    pub last_price: f64,
    pub bid: f64,
    pub ask: f64,
    pub volume: f64,
    pub open_interest: f64,
    pub implied_volatility: f64,  // decimal (0.25 = 25%)
    pub in_the_money: bool,
}

/// OMON — one expiration's call+put chain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptionExpiry {
    pub expiration: String,       // YYYY-MM-DD
    pub days_to_expiry: i64,
    pub calls: Vec<OptionContract>,
    pub puts: Vec<OptionContract>,
}

/// OMON — complete options-chain snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptionsChainSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub underlying_price: f64,
    pub expirations: Vec<OptionExpiry>,
    pub note: String,
}

/// IVOL — one ATM IV observation over time (52-week history bucket).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IvolObservation {
    pub date: String,             // YYYY-MM-DD
    pub atm_iv_pct: f64,
}

/// IVOL — implied-volatility rank and percentile snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IvolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub current_atm_iv_pct: f64,
    pub iv_52w_low_pct: f64,
    pub iv_52w_high_pct: f64,
    pub iv_rank: f64,             // 0..100: (current - low) / (high - low) × 100
    pub iv_percentile: f64,       // 0..100: % of days at or below current
    pub observation_count: usize,
    pub history: Vec<IvolObservation>,
    pub note: String,
}

// ── ADR-116 Godel Parity Round 9 ─────────────────────────────────────────
// SEAG / COR / TRA / TECH / SKEW surfaces — all pure compute over existing
// HP / DVD / OMON caches, zero new API dependencies.

/// SEAG — one month's historical seasonality bucket (Jan..Dec).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeasonalityMonth {
    pub month: u32,               // 1..12
    pub label: String,            // "Jan", "Feb", …
    pub avg_return_pct: f64,      // mean monthly return across years
    pub median_return_pct: f64,
    pub stdev_pct: f64,
    pub positive_years: usize,
    pub total_years: usize,
    pub best_return_pct: f64,
    pub worst_return_pct: f64,
}

/// SEAG — one day-of-week historical bucket (Mon..Fri).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeasonalityDow {
    pub dow: u32,                 // 1..7 (Mon=1, Sun=7)
    pub label: String,            // "Mon", "Tue", …
    pub avg_return_pct: f64,      // mean daily log-return
    pub positive_days: usize,
    pub total_days: usize,
}

/// SEAG — Seasonality analysis snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeasonalitySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub years_covered: usize,
    pub months: Vec<SeasonalityMonth>,
    pub dow: Vec<SeasonalityDow>,
    pub best_month: String,
    pub worst_month: String,
    pub note: String,
}

/// COR — one pairwise correlation cell.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationCell {
    pub peer_symbol: String,
    pub correlation: f64,         // Pearson on daily log-returns
    pub n_observations: usize,
    pub beta_vs_peer: f64,        // slope of ln(subject) vs ln(peer)
}

/// COR — Correlation matrix for a subject vs its peer set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationMatrix {
    pub symbol: String,
    pub as_of: String,
    pub window_days: usize,       // e.g. 252 (1Y)
    pub cells: Vec<CorrelationCell>,
    pub mean_correlation: f64,    // average |ρ| across cells
    pub highest_corr_symbol: String,
    pub lowest_corr_symbol: String,
    pub note: String,
}

/// TRA — one total-return window (price return + dividend yield).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TotalReturnWindow {
    pub label: String,            // "1M" / "3M" / "6M" / "YTD" / "1Y" / "3Y" / "5Y"
    pub trading_days: usize,
    pub price_return_pct: f64,
    pub dividend_yield_pct: f64,  // dividends paid in window / start price × 100
    pub total_return_pct: f64,    // price + dividend yield (simple, not compound)
    pub annualized_pct: f64,      // annualized for windows ≥ 1Y, else simple
    pub dividends_paid: f64,      // cash per share in window
    pub n_dividends: usize,
}

/// TRA — Total return analysis snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TotalReturnSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub last_close: f64,
    pub trailing_12m_dividends: f64,
    pub trailing_12m_yield_pct: f64,
    pub windows: Vec<TotalReturnWindow>,
    pub note: String,
}

/// TECH — one indicator value with its signal interpretation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TechnicalIndicator {
    pub name: String,             // "RSI(14)", "MACD(12,26,9)", "BB(20,2)", "ATR(14)", "ADX(14)", "Stoch(14,3)"
    pub value: f64,               // primary value (for MACD this is the histogram)
    pub value_secondary: f64,     // signal line / middle band / +DI / etc.
    pub value_tertiary: f64,      // -DI / lower band / …
    pub signal: String,           // "overbought" / "oversold" / "bullish" / "bearish" / "neutral"
    pub note: String,             // short contextual hint
}

/// TECH — Technical indicator snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TechnicalSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub last_close: f64,
    pub indicators: Vec<TechnicalIndicator>,
    pub trend_summary: String,    // short synthesized label
    pub note: String,
}

/// SKEW — one strike row on a volatility smile curve.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkewPoint {
    pub strike: f64,
    pub moneyness_pct: f64,       // (strike / underlying - 1) × 100
    pub call_iv_pct: f64,
    pub put_iv_pct: f64,
    pub combined_iv_pct: f64,     // average of call/put when both present
}

/// SKEW — one expiry's full smile + summary stats.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkewExpiry {
    pub expiration: String,
    pub days_to_expiry: i64,
    pub atm_iv_pct: f64,
    pub points: Vec<SkewPoint>,
    pub put_call_skew_25d_pct: f64, // 25-delta put IV − 25-delta call IV (placeholder using ±10% OTM)
    pub term_note: String,
}

/// SKEW — Implied-volatility skew snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolatilitySkew {
    pub symbol: String,
    pub as_of: String,
    pub underlying_price: f64,
    pub expiries: Vec<SkewExpiry>,
    pub note: String,
}

// ── ADR-117 Godel Parity Round 10 ───────────────────────────────────────────

/// LEV — one leverage / coverage ratio row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeverageRatio {
    pub name: String,
    pub value: f64,
    pub peer_median: f64,      // 0.0 when unknown
    pub signal: String,        // "HEALTHY" | "ELEVATED" | "STRETCHED" | "NEUTRAL"
    pub note: String,
}

/// LEV — full leverage / solvency snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeverageSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub total_debt: f64,
    pub net_debt: f64,
    pub ebitda_ttm: f64,
    pub interest_expense_ttm: f64,
    pub total_equity: f64,
    pub ratios: Vec<LeverageRatio>,
    pub solvency_summary: String,
    pub note: String,
}

/// ACRL — one quarter's earnings-quality row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccrualPeriod {
    pub period: String,          // "FY2024" or "Q3 2024"
    pub date: String,            // YYYY-MM-DD
    pub net_income: f64,
    pub free_cash_flow: f64,
    pub fcf_to_ni_ratio: f64,    // FCF / NI
    pub cash_conversion_pct: f64, // FCF / NI × 100
    pub accruals: f64,           // NI - FCF
    pub quality_label: String,   // "HIGH" | "MEDIUM" | "LOW" | "NEGATIVE_NI"
}

/// ACRL — earnings quality snapshot (accruals vs cash flow conversion).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccrualsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub ttm_net_income: f64,
    pub ttm_free_cash_flow: f64,
    pub ttm_cash_conversion_pct: f64,
    pub avg_cash_conversion_pct: f64,    // across the tracked periods
    pub periods: Vec<AccrualPeriod>,
    pub trend_label: String,             // "IMPROVING" | "STABLE" | "DETERIORATING" | "MIXED"
    pub note: String,
}

/// RVOL — one realized-volatility window observation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealizedVolWindow {
    pub label: String,       // "20d" / "60d" / "120d" / "252d"
    pub trading_days: usize,
    pub realized_vol_pct: f64,   // annualized
    pub percentile: f64,         // 0..=100 — cone rank vs the full history of this window
    pub n_observations: usize,
}

/// RVOL — realized volatility + IV/RV gap snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealizedVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub last_close: f64,
    pub current_atm_iv_pct: f64,      // from cached IVOL, 0.0 when unknown
    pub iv_rv_gap_pct: f64,           // IV − RV(20d)
    pub iv_rv_ratio: f64,             // IV / RV(20d)
    pub windows: Vec<RealizedVolWindow>,
    pub regime_label: String,         // "CHEAP_IV" | "FAIR_IV" | "RICH_IV" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// FCFY — one dividend coverage / FCF yield row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FcfYieldPeriod {
    pub period: String,
    pub date: String,
    pub free_cash_flow: f64,
    pub dividends_paid: f64,
    pub payout_from_fcf_pct: f64,   // dividends_paid / FCF × 100 (absolute cash-out ratio)
    pub payout_from_ni_pct: f64,    // dividends_paid / NI × 100
    pub fcf_yield_pct: f64,         // FCF / market_cap_at_period × 100 (only TTM-level rows populate this)
}

/// FCFY — FCF yield + dividend sustainability snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FcfYieldSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub market_cap: f64,
    pub ttm_free_cash_flow: f64,
    pub ttm_dividends_paid: f64,
    pub ttm_fcf_yield_pct: f64,
    pub ttm_dividend_yield_pct: f64,
    pub ttm_payout_from_fcf_pct: f64,
    pub ttm_payout_from_ni_pct: f64,
    pub fcf_cagr_5y_pct: f64,       // 0.0 when <5 years of annuals
    pub periods: Vec<FcfYieldPeriod>,
    pub sustainability_label: String,   // "SAFE" | "STRETCHED" | "UNSUSTAINABLE" | "NO_DIVIDEND"
    pub note: String,
}

/// SHRT — short interest + days-to-cover + squeeze signal snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShortInterestSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub shares_outstanding: f64,
    pub shares_float: f64,
    pub short_shares: f64,
    pub short_percent_of_float: f64,
    pub avg_daily_volume_20d: f64,
    pub days_to_cover: f64,             // short_shares / avg_daily_volume_20d
    pub short_ratio_reported: f64,      // from Fundamentals (vendor-provided, may differ)
    pub utilization_proxy_pct: f64,     // short / float × 100 (same as short_percent_of_float but normalized)
    pub squeeze_risk_label: String,     // "LOW" | "ELEVATED" | "HIGH" | "EXTREME" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── ADR-118 Godel Parity Round 11 ───────────────────────────────────────────

/// ALTZ — one component of the Altman Z-score.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AltmanComponent {
    pub name: String,               // e.g. "A: WC/TA"
    pub ratio: f64,                 // raw ratio value
    pub coefficient: f64,           // 1.2 / 1.4 / 3.3 / 0.6 / 1.0
    pub contribution: f64,          // coefficient × ratio
    pub note: String,
}

/// ALTZ — Altman Z-score snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AltmanZSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub working_capital: f64,
    pub retained_earnings: f64,
    pub ebit: f64,
    pub market_value_equity: f64,
    pub sales: f64,
    pub total_assets: f64,
    pub total_liabilities: f64,
    pub z_score: f64,               // sum of all contributions
    pub zone: String,               // "DISTRESS" (<1.81) | "GRAY" | "SAFE" (>=2.99) | "INSUFFICIENT_DATA"
    pub components: Vec<AltmanComponent>,
    pub note: String,
}

/// PTFS — one Piotroski F-score check with signal.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PiotroskiCheck {
    pub category: String,           // "Profitability" | "Leverage/Liquidity" | "Operating Efficiency"
    pub name: String,
    pub passed: bool,
    pub value_current: f64,
    pub value_prior: f64,
    pub note: String,
}

/// PTFS — Piotroski F-score snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PiotroskiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub current_period: String,
    pub prior_period: String,
    pub f_score: i32,               // 0..9
    pub strength_label: String,     // "STRONG" (>=7) | "MIXED" | "WEAK" (<=3) | "INSUFFICIENT_DATA"
    pub profitability_score: i32,   // 0..4
    pub leverage_score: i32,        // 0..3
    pub efficiency_score: i32,      // 0..2
    pub checks: Vec<PiotroskiCheck>,
    pub note: String,
}

/// VOLE — one volatility estimator row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolEstimator {
    pub name: String,               // "ClosedToClose" / "Parkinson" / "GarmanKlass" / "RogersSatchell" / "YangZhang"
    pub annualized_vol_pct: f64,
    pub efficiency_vs_close: f64,   // multiplicative gain vs close-to-close (1.0 = same)
    pub note: String,
}

/// VOLE — OHLC volatility estimator snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OhlcVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub trading_days: usize,
    pub estimators: Vec<VolEstimator>,
    pub preferred_estimate_pct: f64,  // Yang-Zhang when all 4 available, else Parkinson, else CtC
    pub preferred_label: String,
    pub note: String,
}

/// EPSB — EPS beat streak & surprise analysis snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EpsBeatSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub total_reports: usize,
    pub beats: usize,
    pub misses: usize,
    pub inlines: usize,
    pub beat_rate_pct: f64,         // beats / total × 100
    pub current_streak: i32,        // positive = beat streak, negative = miss streak
    pub longest_beat_streak: usize,
    pub longest_miss_streak: usize,
    pub avg_surprise_pct: f64,
    pub median_surprise_pct: f64,
    pub recent_avg_surprise_pct: f64,  // last 4 reports
    pub bias_label: String,         // "POSITIVE" | "NEGATIVE" | "NEUTRAL"
    pub trend_label: String,        // "ACCELERATING" | "STABLE" | "DECELERATING"
    pub latest_date: String,
    pub latest_surprise_pct: f64,
    pub note: String,
}

/// PTD — Price Target Dispersion & Implied Return snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PriceTargetDispersion {
    pub symbol: String,
    pub as_of: String,
    pub current_price: f64,
    pub target_high: f64,
    pub target_low: f64,
    pub target_mean: f64,
    pub target_median: f64,
    pub num_analysts: i32,
    pub dispersion_pct: f64,        // (high - low) / mean × 100
    pub spread_pct: f64,            // (high - low) / current × 100
    pub implied_return_median_pct: f64,
    pub implied_return_mean_pct: f64,
    pub upside_to_high_pct: f64,
    pub downside_to_low_pct: f64,
    pub consensus_label: String,    // "BULLISH" | "NEUTRAL" | "BEARISH" | "NO_COVERAGE"
    pub note: String,
}

// ── ADR-119 Godel Parity Round 12 ───────────────────────────────────────────

/// MNGR — Insider Activity Bias snapshot for a symbol.
/// Computed from cached INS (Form 4 insider trades) within a lookback window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsiderActivitySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub window_days: i32,
    pub total_trades: usize,
    pub buy_count: usize,
    pub sell_count: usize,
    pub other_count: usize,         // awards, exercises, etc.
    pub unique_insiders: usize,
    pub gross_buy_value_usd: f64,
    pub gross_sell_value_usd: f64,
    pub net_value_usd: f64,         // buy - sell
    pub buy_sell_ratio: f64,        // buy_count / max(sell_count, 1)
    pub net_shares: f64,            // buy_shares - sell_shares
    pub latest_trade_date: String,
    pub bias_label: String,         // "BULLISH" | "NEUTRAL" | "BEARISH" | "NO_ACTIVITY"
    pub conviction_label: String,   // "HIGH" | "MEDIUM" | "LOW" | "NONE"
    pub note: String,
}

/// DIVG — one annual-bucket dividend aggregation row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DivgAnnualRow {
    pub year: i32,
    pub total_amount: f64,          // sum of cash dividends in the calendar year
    pub payment_count: usize,
    pub growth_pct: f64,            // yoy % change vs prior year (0 if prior = 0)
}

/// DIVG — Dividend Growth Analysis snapshot.
/// Computed from cached DVD historical dividend payments.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DivgSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub total_payments: usize,
    pub first_payment_date: String,
    pub latest_payment_date: String,
    pub latest_amount: f64,
    pub annualized_dividend: f64,    // sum of most recent 4 payments
    pub years_covered: usize,
    pub cagr_1y_pct: f64,           // year-over-year growth (latest annual bucket)
    pub cagr_3y_pct: f64,           // 3-year CAGR
    pub cagr_5y_pct: f64,           // 5-year CAGR
    pub consecutive_growth_years: usize,
    pub consistency_score_pct: f64, // % of yoy deltas that are non-negative
    pub annual_rows: Vec<DivgAnnualRow>,
    pub trend_label: String,        // "GROWING" | "STABLE" | "CUTTING" | "NO_HISTORY"
    pub note: String,
}

/// EARM — one quarterly momentum row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarmQuarterRow {
    pub period: String,             // "YYYY-MM-DD"
    pub revenue: f64,
    pub revenue_yoy_pct: f64,       // vs year-ago quarter (same position + 4)
    pub eps_actual: f64,
    pub eps_estimate: f64,
    pub eps_surprise_pct: f64,
}

/// EARM — Earnings Momentum Trend snapshot.
/// Computed from cached FA (quarterly income statements) + EPS (surprise history).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub quarters_used: usize,
    pub recent_revenue_growth_pct: f64, // avg yoy of latest 4 Qs
    pub prior_revenue_growth_pct: f64,  // avg yoy of prior 4 Qs
    pub revenue_acceleration_pct: f64,  // recent - prior
    pub recent_eps_surprise_pct: f64,   // avg surprise % of latest 4 reports
    pub prior_eps_surprise_pct: f64,    // avg surprise % of prior 4 reports
    pub eps_surprise_acceleration_pct: f64,
    pub composite_score: f64,           // 0..100 blended momentum score
    pub momentum_label: String,         // "ACCELERATING" | "STABLE" | "DECELERATING" | "INSUFFICIENT_DATA"
    pub quarters: Vec<EarmQuarterRow>,
    pub note: String,
}

/// SECTR — Sector Rotation Strength snapshot for a symbol.
/// Computed from cached INDU (current sector % changes) + symbol's sector field.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SectorRotationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub symbol_sector: String,
    pub symbol_sector_change_pct: f64,
    pub sector_rank: i32,               // 1 = strongest, N = weakest
    pub sectors_total: i32,
    pub avg_sector_change_pct: f64,
    pub median_sector_change_pct: f64,
    pub relative_strength_pct: f64,     // sector - avg
    pub breadth_pct: f64,               // % of sectors with positive change
    pub strongest_sector: String,
    pub strongest_sector_pct: f64,
    pub weakest_sector: String,
    pub weakest_sector_pct: f64,
    pub strength_label: String,         // "LEADER" | "NEUTRAL" | "LAGGARD" | "NO_DATA"
    pub note: String,
}

/// UPDM — Upgrade/Downgrade Momentum snapshot for a symbol.
/// Computed from cached UPDG (RatingChange history).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpdmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub total_actions: usize,
    pub upgrades_30d: usize,
    pub downgrades_30d: usize,
    pub upgrades_90d: usize,
    pub downgrades_90d: usize,
    pub upgrades_180d: usize,
    pub downgrades_180d: usize,
    pub initiations_90d: usize,
    pub maintains_90d: usize,
    pub net_30d: i32,                   // upgrades - downgrades, 30d window
    pub net_90d: i32,
    pub net_180d: i32,
    pub latest_date: String,
    pub latest_action: String,          // "upgrade" / "downgrade" / "initiation" / "maintain"
    pub latest_firm: String,
    pub latest_to_grade: String,
    pub bias_label: String,             // "BULLISH" | "NEUTRAL" | "BEARISH" | "NO_COVERAGE"
    pub trend_label: String,            // "IMPROVING" | "STABLE" | "DETERIORATING"
    pub note: String,
}

// ── ADR-120 Godel Parity Round 13 ───────────────────────────────────────────

/// MOM — 12-1 month momentum snapshot for a symbol.
/// Pure compute over cached historical bars (HP).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MomentumSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: i32,
    pub return_1m_pct: f64,
    pub return_3m_pct: f64,
    pub return_6m_pct: f64,
    pub return_12m_pct: f64,
    pub return_12_1_pct: f64,        // 12-month minus 1-month
    pub vol_annualized_pct: f64,     // daily stdev × √252
    pub vol_adjusted_score: f64,     // return_12_1 / vol_annualized
    pub composite_score: f64,        // 0..100 composite
    pub regime_label: String,        // "STRONG" | "NEUTRAL" | "WEAK" | "CRASH" | "INSUFFICIENT_DATA"
    pub trend_label: String,         // "ACCELERATING" | "STABLE" | "DECELERATING"
    pub note: String,
}

/// LIQ — Liquidity profile snapshot for a symbol.
/// Pure compute over cached historical bars (HP) + Fundamentals shares_outstanding.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiquiditySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub window_days: i32,
    pub avg_daily_share_volume: f64,
    pub median_daily_share_volume: f64,
    pub avg_daily_dollar_volume: f64,
    pub median_daily_dollar_volume: f64,
    pub shares_outstanding: f64,
    pub daily_turnover_pct: f64,            // avg share volume / shares out × 100
    pub amihud_illiquidity: f64,            // 1e6 × mean(|return| / dollar volume)
    pub avg_true_range_pct: f64,            // mean((high-low)/close) × 100
    pub spread_proxy_pct: f64,              // Corwin-Schultz high-low estimator
    pub liquidity_tier: String,             // "DEEP" | "LIQUID" | "MODERATE" | "THIN" | "ILLIQUID" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// BREAK — Breakout proximity snapshot for a symbol.
/// Pure compute over cached historical bars (HP).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BreakoutSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub current_price: f64,
    pub high_20d: f64,
    pub low_20d: f64,
    pub high_60d: f64,
    pub low_60d: f64,
    pub high_52w: f64,
    pub low_52w: f64,
    pub dist_from_52w_high_pct: f64,    // (current - high) / high × 100 (negative when below)
    pub dist_from_52w_low_pct: f64,
    pub dist_from_20d_high_pct: f64,
    pub dist_from_60d_high_pct: f64,
    pub position_in_52w_range_pct: f64, // (current - low) / (high - low) × 100
    pub position_in_20d_range_pct: f64,
    pub consolidation_pct: f64,         // 20d range / mean × 100
    pub breakout_label: String,         // "NEW_HIGH" | "NEAR_HIGH" | "MID_RANGE" | "NEAR_LOW" | "NEW_LOW"
    pub setup_label: String,            // "BREAKOUT_IMMINENT" | "CONSOLIDATING" | "TRENDING_UP" | "TRENDING_DOWN" | "NEUTRAL"
    pub note: String,
}

/// CCRL — Cash conversion cycle per-period row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CashCycleRow {
    pub period: String,
    pub dso_days: f64,
    pub dio_days: f64,
    pub dpo_days: f64,
    pub ccc_days: f64,
}

/// CCRL — Cash conversion cycle snapshot for a symbol.
/// Pure compute over cached FA statements (annual preferred, quarterly fallback).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CashCycleSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub latest_period: String,
    pub dso_days: f64,
    pub dio_days: f64,
    pub dpo_days: f64,
    pub ccc_days: f64,
    pub prior_ccc_days: f64,
    pub ccc_change_days: f64,
    pub ccc_3y_avg_days: f64,
    pub periods_used: usize,
    pub efficiency_label: String,    // "EFFICIENT" | "NEUTRAL" | "INEFFICIENT" | "INSUFFICIENT_DATA"
    pub trend_label: String,         // "IMPROVING" | "STABLE" | "DETERIORATING"
    pub periods: Vec<CashCycleRow>,
    pub note: String,
}

/// CREDIT — Unified credit score component row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreditComponent {
    pub name: String,
    pub value: String,
    pub score: f64,
    pub weight: f64,
    pub contribution: f64,
}

/// CREDIT — Unified credit score snapshot for a symbol.
/// Fuses cached ALTZ + PTFS + LEV + ACRL snapshots from Rounds 10/11.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CreditSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub altman_z: f64,
    pub altman_zone: String,
    pub piotroski_score: i32,
    pub piotroski_label: String,
    pub leverage_summary: String,
    pub leverage_score: f64,
    pub accruals_trend: String,
    pub accruals_ttm_cash_conversion_pct: f64,
    pub composite_score: f64,        // 0..100
    pub letter_grade: String,        // "AAA" | "AA" | "A" | "BBB" | "BB" | "B" | "CCC" | "INSUFFICIENT_DATA"
    pub credit_label: String,        // "INVESTMENT_GRADE" | "BORDERLINE" | "SPECULATIVE" | "DISTRESSED"
    pub inputs_available: usize,
    pub components: Vec<CreditComponent>,
    pub note: String,
}

// ── ADR-121 Godel Parity Round 14 ───────────────────────────────────────────

/// GROWM — Growth-at-Reasonable-Price (GARP) composite.
/// Fuses cached MOM + EARM + DIVG snapshots from Rounds 12/13.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GarpComponent {
    pub name: String,
    pub value: String,
    pub score: f64,
    pub weight: f64,
    pub contribution: f64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GrowmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub momentum_score: f64,         // from MOM composite
    pub momentum_regime: String,
    pub earnings_momentum_score: f64, // from EARM composite
    pub earnings_label: String,
    pub dividend_cagr_3y_pct: f64,   // from DIVG
    pub dividend_trend: String,
    pub composite_score: f64,        // 0..100
    pub garp_label: String,          // "GARP" | "GROWTH" | "VALUE" | "SPECULATIVE" | "NO_DATA"
    pub inputs_available: usize,
    pub components: Vec<GarpComponent>,
    pub note: String,
}

/// FLOW — Smart-money flow snapshot combining insider + institutional deltas.
/// Computed from cached INS (InsiderTrade) + HDS (InstitutionalHolder) rows.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FlowSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub window_days: i32,
    pub insider_buy_value_usd: f64,
    pub insider_sell_value_usd: f64,
    pub insider_net_value_usd: f64,
    pub insider_trade_count: usize,
    pub unique_insiders: usize,
    pub institutional_share_delta: f64,  // sum of positive+negative HDS changes
    pub institutional_buyers: usize,     // count of holders with change > 0
    pub institutional_sellers: usize,    // count of holders with change < 0
    pub institutional_holders_tracked: usize,
    pub institutional_net_ratio: f64,    // (buyers - sellers) / tracked
    pub insider_score: f64,              // 0..100
    pub institutional_score: f64,        // 0..100
    pub composite_score: f64,            // 0..100 weighted average
    pub flow_label: String,              // "STRONG_BUY" | "BUY" | "NEUTRAL" | "SELL" | "STRONG_SELL" | "NO_DATA"
    pub note: String,
}

/// REGIME — Market regime classifier fusing VOLE + TECH + HRA snapshots.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegimeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub realized_vol_pct: f64,       // from VOLE preferred_estimate_pct
    pub vol_source: String,          // "yang_zhang" | "parkinson" | "close_to_close"
    pub adx_value: f64,              // from TECH (ADX indicator)
    pub trend_summary: String,       // from TECH
    pub sharpe_ratio: f64,           // from HRA
    pub return_1y_pct: f64,          // from HRA
    pub trend_strength_score: f64,   // 0..100 from ADX
    pub volatility_score: f64,       // 0..100 where lower vol = higher score
    pub return_score: f64,           // 0..100 from 1Y return
    pub composite_score: f64,        // 0..100
    pub regime_label: String,        // "TRENDING" | "MEAN_REVERTING" | "VOLATILE" | "QUIET" | "INSUFFICIENT_DATA"
    pub inputs_available: usize,
    pub note: String,
}

/// RELVOL — Relative volume unusual-activity snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub current_volume: f64,
    pub avg_volume_5d: f64,
    pub avg_volume_20d: f64,
    pub avg_volume_60d: f64,
    pub rel_volume_5d: f64,          // current / 5d avg
    pub rel_volume_20d: f64,         // current / 20d avg
    pub rel_volume_60d: f64,         // current / 60d avg
    pub volume_trend_5d_pct: f64,    // (5d avg / 20d avg - 1) × 100
    pub volume_percentile_60d: f64,  // rank of current_volume in the 60d sample, 0..=100
    pub activity_label: String,      // "EXTREME" | "HIGH" | "ELEVATED" | "NORMAL" | "LOW" | "INSUFFICIENT_DATA"
    pub direction_label: String,     // "BULLISH" | "BEARISH" | "NEUTRAL" (from current close vs prior)
    pub bars_used: usize,
    pub note: String,
}

/// MARGINS — Per-period margin row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarginRow {
    pub period: String,
    pub gross_margin_pct: f64,
    pub operating_margin_pct: f64,
    pub net_margin_pct: f64,
}

/// MARGINS — Margin trajectory snapshot.
/// Pure compute over cached FA statements (annual preferred, quarterly fallback).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MarginsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub basis: String,               // "annual" | "quarterly"
    pub latest_period: String,
    pub latest_gross_margin_pct: f64,
    pub latest_operating_margin_pct: f64,
    pub latest_net_margin_pct: f64,
    pub prior_gross_margin_pct: f64,
    pub prior_operating_margin_pct: f64,
    pub prior_net_margin_pct: f64,
    pub gross_margin_change_pct: f64,    // latest - prior, in percentage points
    pub operating_margin_change_pct: f64,
    pub net_margin_change_pct: f64,
    pub avg_gross_margin_pct: f64,       // across tracked periods
    pub avg_operating_margin_pct: f64,
    pub avg_net_margin_pct: f64,
    pub periods_used: usize,
    pub gross_trend_label: String,       // "EXPANDING" | "STABLE" | "CONTRACTING"
    pub operating_trend_label: String,
    pub net_trend_label: String,
    pub overall_trend_label: String,     // majority across the three
    pub quality_label: String,           // "HIGH" | "MEDIUM" | "LOW" (latest op margin bucket)
    pub periods: Vec<MarginRow>,
    pub note: String,
}

// ── ADR-122 Godel Parity Round 15 ───────────────────────────────────────────

/// Generic meta-composite sub-component row used by VAL / QUAL / RISK.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactorComponent {
    pub name: String,
    pub value: String,
    pub score: f64,       // 0..100 (higher = better for VAL/QUAL, higher = riskier for RISK)
    pub weight: f64,      // raw percent weight
    pub contribution: f64,
}

/// VAL — Unified value-factor composite fusing valuation ratios vs sector peers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValueSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,                   // sector used for peer medians
    pub peers_considered: usize,
    // per-metric this-symbol vs sector-median values
    pub pe_ratio: f64,
    pub pe_sector_median: f64,
    pub forward_pe: f64,
    pub forward_pe_sector_median: f64,
    pub price_to_book: f64,
    pub price_to_book_sector_median: f64,
    pub price_to_sales: f64,
    pub price_to_sales_sector_median: f64,
    pub ev_to_ebitda: f64,
    pub ev_to_ebitda_sector_median: f64,
    pub fcf_yield_pct: f64,               // from FCFY snapshot
    pub fcf_yield_sector_median_pct: f64, // sector median of FCFY TTM yield
    pub composite_score: f64,             // 0..100
    pub value_label: String,              // "DEEP_VALUE" | "VALUE" | "FAIR" | "EXPENSIVE" | "PREMIUM" | "NO_DATA"
    pub inputs_available: usize,
    pub components: Vec<FactorComponent>,
    pub note: String,
}

/// QUAL — Unified quality-factor composite fusing PTFS + MARGINS + ACRL + LEV.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QualitySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub piotroski_score: i32,
    pub piotroski_label: String,
    pub operating_margin_pct: f64,
    pub margin_trend_label: String,
    pub cash_conversion_pct: f64,
    pub accruals_trend_label: String,
    pub leverage_summary: String,
    pub debt_to_ebitda: f64,
    pub composite_score: f64,             // 0..100
    pub quality_label: String,            // "HIGH_QUALITY" | "QUALITY" | "AVERAGE" | "POOR" | "WEAK" | "NO_DATA"
    pub inputs_available: usize,
    pub components: Vec<FactorComponent>,
    pub note: String,
}

/// RISK — Unified risk-factor composite fusing VOLE + BETA + LIQ + SHRT + ALTZ.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiskSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub realized_vol_pct: f64,
    pub beta_1y: f64,
    pub liquidity_tier: String,
    pub short_percent_of_float: f64,
    pub days_to_cover: f64,
    pub altman_z: f64,
    pub altman_zone: String,
    pub composite_score: f64,             // 0..100 — higher = RISKIER
    pub risk_label: String,               // "LOW_RISK" | "MODERATE" | "ELEVATED" | "HIGH_RISK" | "DISTRESSED" | "NO_DATA"
    pub inputs_available: usize,
    pub components: Vec<FactorComponent>,
    pub note: String,
}

/// INSSTRK — One per-insider streak row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsiderStreakRow {
    pub insider_name: String,
    pub streak_direction: String,   // "BUY" | "SELL" | "MIXED"
    pub consecutive_events: usize,
    pub net_value_usd: f64,
    pub net_shares: f64,
    pub first_date: String,
    pub latest_date: String,
}

/// INSSTRK — Insider streak detector snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsiderStreakSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub window_days: i32,
    pub unique_insiders: usize,
    pub buy_streak_count: usize,    // insiders with ≥ 2 consecutive buys
    pub sell_streak_count: usize,
    pub longest_buy_streak: usize,
    pub longest_sell_streak: usize,
    pub net_buy_value_usd: f64,
    pub net_sell_value_usd: f64,
    pub streak_label: String,       // "STRONG_ACCUMULATION" | "ACCUMULATION" | "DISTRIBUTION" | "STRONG_DISTRIBUTION" | "MIXED" | "NONE"
    pub rows: Vec<InsiderStreakRow>,
    pub note: String,
}

/// COVG — Analyst coverage breadth + churn snapshot.
/// Fuses cached PriceTarget (coverage size), AnalystRecommendations (consensus
/// distribution), and UPDM (upgrade/downgrade tape) into one snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CoverageSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub num_analysts: i32,
    pub target_mean: f64,
    pub target_low: f64,
    pub target_high: f64,
    pub consensus_strong_buy: i32,
    pub consensus_buy: i32,
    pub consensus_hold: i32,
    pub consensus_sell: i32,
    pub consensus_strong_sell: i32,
    pub consensus_total: i32,
    pub consensus_bull_ratio: f64,  // (strong_buy + buy) / total
    pub upgrades_90d: usize,
    pub downgrades_90d: usize,
    pub net_90d: i32,
    pub churn_90d: usize,           // upgrades + downgrades (total activity)
    pub breadth_score: f64,         // 0..100 (coverage size)
    pub consensus_score: f64,       // 0..100 (bullishness)
    pub churn_score: f64,           // 0..100 (activity)
    pub composite_score: f64,       // 0..100 weighted average
    pub coverage_label: String,     // "EXPANDING" | "STABLE" | "CONTRACTING" | "THIN" | "NONE"
    pub inputs_available: usize,
    pub note: String,
}

// ── ADR-123 Godel Parity Round 16 ───────────────────────────────────────────

/// VRK — Value Rank vs sector peers snapshot.
/// Percentile rank of `ValueSnapshot.composite_score` within the same sector.
/// Higher percentile = better value (label ladder matches VAL's "DEEP_VALUE is good").
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValueRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub composite_score: f64,         // subject's VAL composite (copied)
    pub peers_considered: usize,      // peers in the same sector with a VAL snapshot
    pub peers_with_data: usize,       // same as peers_considered today
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64,         // 0..100 (higher = better value)
    pub rank_position: usize,         // 1-based (1 = best value in cohort)
    pub rank_label: String,           // "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" | "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" | "NO_DATA"
    pub note: String,
}

/// QRK — Quality Rank vs sector peers snapshot.
/// Percentile rank of `QualitySnapshot.composite_score` within the same sector.
/// Higher percentile = higher quality.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QualityRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub composite_score: f64,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64,         // 0..100 (higher = better quality)
    pub rank_position: usize,
    pub rank_label: String,           // same ladder as VRK
    pub note: String,
}

/// RRK — Risk Rank vs sector peers snapshot.
/// Percentile rank of `RiskSnapshot.composite_score` within the same sector.
/// RISK composite is higher = riskier, so this snapshot *inverts* the percentile:
/// higher `percentile_rank` here = SAFER than peers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiskRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub composite_score: f64,         // subject's RISK composite (higher = riskier)
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64,         // 0..100 (higher = SAFER vs peers)
    pub rank_position: usize,         // 1-based (1 = safest in cohort)
    pub rank_label: String,           // "SAFEST_DECILE" | "SAFEST_QUARTILE" | "ABOVE_MEDIAN_SAFE" | "BELOW_MEDIAN_RISKY" | "BOTTOM_QUARTILE_RISKY" | "RISKIEST_DECILE" | "NO_DATA"
    pub note: String,
}

/// RELEPSGR — Relative 3y EPS CAGR vs sector median snapshot.
/// CAGR computed over `FinancialStatements.income_annual[].eps` when at least
/// 4 annual rows exist (latest vs latest-3y = 3-year CAGR).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelativeEpsGrowthSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub latest_eps: f64,
    pub earliest_eps: f64,
    pub years_used: usize,
    pub symbol_cagr_pct: f64,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_cagr_pct: f64,
    pub sector_p25_cagr_pct: f64,
    pub sector_p75_cagr_pct: f64,
    pub gap_to_median_pp: f64,        // symbol_cagr - sector_median (in percentage points)
    pub relative_label: String,       // "FAR_ABOVE" | "ABOVE" | "INLINE" | "BELOW" | "FAR_BELOW" | "CAGR_NEGATIVE" | "NO_DATA"
    pub note: String,
}

/// PEAD — Per-event drift row (one per earnings announcement within the window).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PeadEventRow {
    pub event_date: String,
    pub surprise_pct: f64,
    pub classification: String,       // "BEAT" | "MISS" | "INLINE"
    pub drift_1d_pct: f64,
    pub drift_3d_pct: f64,
    pub drift_5d_pct: f64,
    pub drift_10d_pct: f64,
}

/// PEAD — Post-Earnings-Announcement Drift snapshot.
/// Joins cached `EarningsSurprise` rows with cached `HistoricalPriceRow` bars
/// to measure average forward drift over 1 / 3 / 5 / 10 trading days after
/// each announcement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PeadSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub num_events: usize,            // surprises in the cache
    pub events_used: usize,           // surprises successfully matched to HP bars
    pub avg_drift_1d_pct: f64,
    pub avg_drift_3d_pct: f64,
    pub avg_drift_5d_pct: f64,
    pub avg_drift_10d_pct: f64,
    pub beat_event_drift_5d_pct: f64,
    pub miss_event_drift_5d_pct: f64,
    pub latest_event_date: String,
    pub latest_event_surprise_pct: f64,
    pub latest_event_drift_5d_pct: f64,
    pub drift_direction_label: String, // "DRIFT_UP" | "DRIFT_DOWN" | "MIXED" | "INSUFFICIENT_DATA"
    pub rows: Vec<PeadEventRow>,
    pub note: String,
}

// ── ADR-124 Round 17 — size/momentum/drift rank + ops quality + revenue growth ──

/// SIZEF — Size Factor Rank snapshot.
/// Percentile rank of `Fundamentals.market_cap` within the same sector,
/// plus a tier label derived from absolute market cap.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SizeFactorSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub market_cap: f64,              // subject's market cap (USD)
    pub log_market_cap: f64,          // ln(market_cap); 0 if cap <= 0
    pub tier_label: String,           // "MEGA_CAP" | "LARGE_CAP" | "MID_CAP" | "SMALL_CAP" | "MICRO_CAP" | "NO_DATA"
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_cap: f64,
    pub sector_p25_cap: f64,
    pub sector_p75_cap: f64,
    pub percentile_rank: f64,         // 0..100 (higher = larger within sector)
    pub rank_position: usize,         // 1-based (1 = largest)
    pub rank_label: String,           // decile ladder — "TOP_DECILE" .. "BOTTOM_DECILE" | "NO_DATA"
    pub note: String,
}

/// MOMF — Momentum Factor Rank snapshot.
/// Percentile rank of `MomentumSnapshot.composite_score` within the same sector.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MomentumRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub composite_score: f64,         // subject's MOM composite (copied)
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64,         // 0..100 (higher = stronger momentum)
    pub rank_position: usize,         // 1-based (1 = strongest)
    pub rank_label: String,           // same decile ladder as VRK/QRK
    pub note: String,
}

/// PEADRANK — Post-Earnings Drift Rank snapshot.
/// Percentile rank of `PeadSnapshot.avg_drift_5d_pct` within the same sector,
/// restricted to peers whose PEAD snapshot has `drift_direction_label !=
/// "INSUFFICIENT_DATA"` and `events_used >= 3`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PeadRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub avg_drift_5d_pct: f64,        // subject's avg 5d drift (copied)
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_drift_5d_pct: f64,
    pub sector_p25_drift_5d_pct: f64,
    pub sector_p75_drift_5d_pct: f64,
    pub percentile_rank: f64,         // 0..100 (higher = stronger positive drift)
    pub rank_position: usize,         // 1-based (1 = strongest drift-up)
    pub rank_label: String,           // same decile ladder as VRK
    pub note: String,
}

/// FQM — Fundamental Quality Meter snapshot.
/// One-layer composite over raw Round 10 caches (PTFS, MARGINS, ACRL),
/// intentionally **excluding** leverage so the signal measures
/// operational cash-machine health rather than balance-sheet strength.
/// Distinct from QUAL which weighs LEV at 20 %.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FundamentalQualityMeterSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub piotroski_score: i32,         // 0..9
    pub piotroski_label: String,
    pub operating_margin_pct: f64,
    pub margin_trend_label: String,   // EXPANDING / STABLE / CONTRACTING / MIXED
    pub cash_conversion_pct: f64,     // TTM cash conversion
    pub accruals_trend_label: String, // HIGH / STABLE / LOW / DETERIORATING
    pub composite_score: f64,         // 0..100
    pub operator_label: String,       // "ELITE_OPERATOR" | "STRONG_OPERATOR" | "AVERAGE_OPERATOR" | "WEAK_OPERATOR" | "BROKEN_OPERATOR" | "NO_DATA"
    pub inputs_available: i32,        // 0..3 (PTFS/MARGINS/ACRL)
    pub components: Vec<FactorComponent>, // 3 rows
    pub note: String,
}

/// REVRANK — Relative Revenue Growth Rank snapshot.
/// 3-year revenue CAGR compared to sector median CAGR.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RevenueGrowthRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub latest_revenue: f64,
    pub earliest_revenue: f64,
    pub years_used: usize,
    pub symbol_cagr_pct: f64,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_cagr_pct: f64,
    pub sector_p25_cagr_pct: f64,
    pub sector_p75_cagr_pct: f64,
    pub gap_to_median_pp: f64,        // symbol_cagr - sector_median
    pub relative_label: String,       // "FAR_ABOVE" | "ABOVE" | "INLINE" | "BELOW" | "FAR_BELOW" | "CAGR_NEGATIVE" | "NO_DATA"
    pub note: String,
}

// ── ADR-125 Round 18 — rank overlays + surprise streak ────────────────────

/// LEVRANK — Leverage Rank vs Sector Peers.
/// Percentile rank of debt-to-equity (`total_debt / total_equity`) from the
/// cached `LeverageSnapshot`, within the same sector. Inverted — lower D/E
/// = safer = higher rank. Uses RRK-style SAFEST label ladder.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeverageRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub debt_to_equity: f64,          // subject's D/E (0 when equity non-positive)
    pub total_debt: f64,
    pub total_equity: f64,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_d2e: f64,
    pub sector_p25_d2e: f64,
    pub sector_p75_d2e: f64,
    pub percentile_rank: f64,         // 0..100 (higher = SAFER, lower D/E)
    pub rank_position: usize,         // 1-based (1 = safest)
    pub rank_label: String,           // "SAFEST_DECILE" / ... / "RISKIEST_DECILE" / "NEGATIVE_EQUITY" / "NO_DATA"
    pub note: String,
}

/// OPERANK — Operating Quality Rank vs Sector Peers.
/// Percentile rank of `MarginsSnapshot.latest_operating_margin_pct` within
/// the same sector. Higher operating margin = higher rank.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperatingQualityRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub operating_margin_pct: f64,    // subject's latest op margin
    pub margin_trend_label: String,   // copied from MarginsSnapshot.overall_trend_label
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_margin_pct: f64,
    pub sector_p25_margin_pct: f64,
    pub sector_p75_margin_pct: f64,
    pub percentile_rank: f64,         // 0..100 (higher = fatter margins)
    pub rank_position: usize,         // 1-based (1 = fattest)
    pub rank_label: String,           // standard decile ladder
    pub note: String,
}

/// FQMRANK — Fundamental Quality Meter Rank vs Sector Peers.
/// Percentile rank of `FundamentalQualityMeterSnapshot.composite_score`
/// within the same sector.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FqmRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub composite_score: f64,         // subject's FQM composite (copied)
    pub operator_label: String,       // subject's FQM operator label (copied)
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64,         // 0..100 (higher = better operator)
    pub rank_position: usize,         // 1-based (1 = best operator)
    pub rank_label: String,           // standard decile ladder
    pub note: String,
}

/// LIQRANK — Liquidity Rank vs Sector Peers.
/// Percentile rank of `LiquiditySnapshot.avg_daily_dollar_volume` within the
/// same sector. Higher dollar volume = deeper = higher rank. The subject's
/// `liquidity_tier` label is copied for reference.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LiquidityRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub avg_daily_dollar_volume: f64, // subject's ADV$ (copied)
    pub tier_label: String,           // subject's LIQ tier (copied, e.g. "DEEP" / "LIQUID" / ...)
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_dollar_volume: f64,
    pub sector_p25_dollar_volume: f64,
    pub sector_p75_dollar_volume: f64,
    pub percentile_rank: f64,         // 0..100 (higher = deeper liquidity)
    pub rank_position: usize,         // 1-based (1 = deepest)
    pub rank_label: String,           // standard decile ladder
    pub note: String,
}

/// SURPSTK — Earnings Surprise Streak snapshot.
/// Pure time-series stat over cached `EarningsSurprise` rows: counts
/// consecutive beats/misses, computes beat rate over the sample window,
/// and emits a streak-strength label. No sector needed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarningsSurpriseStreakSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub total_events: usize,          // events considered (BEAT/MISS/INLINE classification)
    pub beats: usize,
    pub misses: usize,
    pub inlines: usize,
    pub beat_rate_pct: f64,           // beats / total_events × 100
    pub current_streak_type: String,  // "BEAT" | "MISS" | "INLINE" | "NONE"
    pub current_streak_len: usize,    // consecutive length of current streak
    pub longest_beat_streak: usize,
    pub longest_miss_streak: usize,
    pub avg_surprise_pct: f64,
    pub latest_event_date: String,
    pub latest_event_surprise_pct: f64,
    pub latest_event_label: String,   // "BEAT" | "MISS" | "INLINE"
    pub streak_label: String,         // "HOT_STREAK" | "BEAT_TREND" | "MIXED" | "MISS_TREND" | "COLD_STREAK" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── ADR-126 Round 19 — dividend/earnings/rating rank overlays + gap/streak ─

/// DVDRANK — Dividend Growth Rank vs Sector Peers.
/// Percentile rank of `DivgSnapshot.cagr_3y_pct` within the same sector.
/// Higher CAGR = higher rank.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DividendGrowthRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub cagr_3y_pct: f64,             // subject's 3y dividend CAGR (copied from DIVG)
    pub consecutive_growth_years: usize,
    pub trend_label: String,          // subject's DIVG trend (copied, e.g. "GROWING")
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_cagr_pct: f64,
    pub sector_p25_cagr_pct: f64,
    pub sector_p75_cagr_pct: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String,           // standard decile ladder
    pub note: String,
}

/// EARMRANK — Earnings Momentum Rank vs Sector Peers.
/// Percentile rank of `EarmSnapshot.composite_score` within the same sector.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarningsMomentumRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub composite_score: f64,
    pub momentum_label: String,       // subject's EARM label (copied)
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String,
    pub note: String,
}

/// UPDGRANK — Upgrade/Downgrade Rank vs Sector Peers.
/// Percentile rank of `UpdmSnapshot.net_90d` within the same sector. A higher
/// net (more upgrades than downgrades) earns a higher rank. No-coverage peers
/// are filtered out so the cohort captures sell-side conviction only.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UpgradeDowngradeRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub net_90d: i32,                 // subject's UPDM net_90d (copied)
    pub bias_label: String,           // subject's UPDM bias (copied)
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_net_90d: f64,
    pub sector_p25_net_90d: f64,
    pub sector_p75_net_90d: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String,
    pub note: String,
}

/// GY — Gap Yearly snapshot. Pure time-series stat over the cached HP daily
/// bars. Counts overnight gaps (today's open vs yesterday's close) binned by
/// magnitude, and emits a "gappiness" label.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GapYearlySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,             // sessions actually scanned (<=252)
    pub gaps_total: usize,            // non-zero gaps seen
    pub gaps_up_2pct: usize,          // |gap| >= 2% and positive
    pub gaps_down_2pct: usize,
    pub gaps_up_5pct: usize,
    pub gaps_down_5pct: usize,
    pub gaps_up_10pct: usize,
    pub gaps_down_10pct: usize,
    pub largest_up_gap_pct: f64,      // biggest positive gap seen (signed)
    pub largest_up_gap_date: String,
    pub largest_down_gap_pct: f64,    // biggest negative gap seen (signed, negative)
    pub largest_down_gap_date: String,
    pub avg_abs_gap_pct: f64,         // mean |gap| across all non-zero gaps
    pub gap_label: String,            // "EXPLOSIVE" | "GAPPY" | "NORMAL" | "SMOOTH" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// DES — Daily Event Streak snapshot. Pure time-series stat over the cached
/// HP daily bars. Tracks the current up/down close-over-close streak, the
/// longest up and down streaks in the window, plus a directional bias label.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyEventStreakSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,             // sessions actually scanned (<=252)
    pub current_streak_type: String,  // "UP" | "DOWN" | "FLAT" | "NONE"
    pub current_streak_len: usize,
    pub longest_up_streak: usize,
    pub longest_down_streak: usize,
    pub up_days: usize,
    pub down_days: usize,
    pub flat_days: usize,
    pub up_day_rate_pct: f64,         // up_days / (up+down) × 100
    pub avg_up_move_pct: f64,         // mean % change on up days
    pub avg_down_move_pct: f64,
    pub streak_label: String,         // "STRONG_UPTREND" | "UPTREND_BIAS" | "NEUTRAL" | "DOWNTREND_BIAS" | "STRONG_DOWNTREND" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── ADR-127 Round 20 — yield/short rank + HP volatility/drawdown/returns ──

/// DVDYIELDRANK — Dividend Yield Rank vs Sector Peers.
/// Percentile rank of `Fundamentals.dividend_yield` within the same sector.
/// Non-payers (`dividend_yield.is_none() || dividend_yield == 0.0`) are
/// filtered out so the cohort captures dividend-paying names only.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DividendYieldRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub dividend_yield_pct: f64,      // subject's current dividend yield %
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_yield_pct: f64,
    pub sector_p25_yield_pct: f64,
    pub sector_p75_yield_pct: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String,           // standard decile ladder
    pub note: String,
}

/// SHRANK — Short Interest Rank vs Sector Peers.
/// Percentile rank of `Fundamentals.short_percent_of_float` within the same
/// sector, risk-inverted so a *lower* short interest earns a *higher* (safer)
/// rank. Names with no short interest data are filtered out.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShortInterestRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub short_pct_of_float: f64,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_short_pct: f64,
    pub sector_p25_short_pct: f64,
    pub sector_p75_short_pct: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String,           // risk-inverted: SAFEST_DECILE (lowest short) → RISKIEST_DECILE
    pub note: String,
}

/// ATRANN — Annualized ATR (Volatility Regime).
/// Pure symbol-local time-series stat over the cached HP daily bars. Computes
/// the 14-period Average True Range (Wilder) on the most recent 253 sessions,
/// annualizes via √252, and maps to a volatility regime label.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnnualizedAtrSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,             // sessions in the window (<=253)
    pub latest_close: f64,
    pub atr14: f64,                   // 14-period Wilder ATR in price units
    pub atr14_pct: f64,               // atr14 / latest_close × 100
    pub atr_annualized_pct: f64,      // atr14_pct × √252
    pub regime_label: String,         // "LOW_VOL" | "NORMAL_VOL" | "HIGH_VOL" | "EXTREME_VOL" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// DDHIST — Drawdown History.
/// Pure symbol-local time-series stat over the same HP window. Tracks the
/// maximum drawdown (deepest peak-to-trough decline), the longest drawdown
/// duration (sessions from peak to recovery), the number of 5% corrections
/// (local peaks followed by 5%+ declines), and the current drawdown from the
/// running peak.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawdownHistorySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub max_drawdown_pct: f64,        // deepest drawdown in the window (negative)
    pub max_drawdown_peak_date: String,
    pub max_drawdown_trough_date: String,
    pub longest_drawdown_days: usize, // sessions from peak to recovery (or to end of window if unrecovered)
    pub corrections_5pct: usize,      // count of local-peak-to-trough declines ≥5%
    pub corrections_10pct: usize,     // count of local-peak-to-trough declines ≥10%
    pub current_drawdown_pct: f64,    // latest close vs running peak (negative or 0)
    pub regime_label: String,         // "RECOVERING" | "SHALLOW" | "MEANINGFUL" | "SEVERE" | "CATASTROPHIC" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// PRICEPERF — Multi-horizon Price Performance.
/// Pure symbol-local time-series stat over the HP cache. Computes total
/// returns at 1M (21 sessions), 3M (63), 6M (126), YTD (since Jan 1 of
/// as_of's year), and 1Y (253) lookbacks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PricePerformanceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub latest_close: f64,
    pub ret_1m_pct: f64,              // % change over trailing 21 sessions
    pub ret_3m_pct: f64,
    pub ret_6m_pct: f64,
    pub ret_ytd_pct: f64,             // % change from first session of as_of's year
    pub ret_1y_pct: f64,              // % change over trailing 253 sessions
    pub trend_label: String,          // "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── ADR-128 Round 21 — beta/peg rank + HP 52wk/rvcone/calendar ──

/// BETARANK — Sector percentile rank of Fundamentals.beta, risk-inverted.
/// Lower beta earns a higher (safer) rank, mirroring SHRANK / LEVRANK /
/// RRK. Requires ≥3 sector peers with a non-None beta value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BetaRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub subject_beta: Option<f64>,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_beta: f64,
    pub sector_p25_beta: f64,
    pub sector_p75_beta: f64,
    pub percentile_rank: f64,         // risk-inverted: low beta → high pct
    pub rank_position: usize,         // 1 = safest beta in sector
    pub rank_label: String,           // SAFEST_DECILE … RISKIEST_DECILE | INSUFFICIENT_DATA | NO_DATA
    pub note: String,
}

/// PEGRANK — Sector percentile rank of Fundamentals.peg_ratio.
/// Lower PEG (cheaper growth) earns a higher (better-value) rank. Not
/// covered by VAL (which uses P/E, Forward P/E, P/B, P/S, EV/EBITDA, FCFY).
/// Requires ≥3 sector peers with a positive finite peg_ratio.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PegRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub subject_peg: Option<f64>,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_peg: f64,
    pub sector_p25_peg: f64,
    pub sector_p75_peg: f64,
    pub percentile_rank: f64,         // value-inverted: low PEG → high pct
    pub rank_position: usize,         // 1 = best value in sector
    pub rank_label: String,           // TOP_DECILE … BOTTOM_DECILE | INSUFFICIENT_DATA | NO_DATA
    pub note: String,
}

/// FHIGHLOW — 52-week high/low distance + proximity band.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Tracks max/min close + dates + current-vs-high/low distance + a
/// proximity label (AT_HIGH / NEAR_HIGH / MID_RANGE / NEAR_LOW / AT_LOW).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FiftyTwoWeekHighLowSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub latest_close: f64,
    pub high_52w: f64,
    pub high_52w_date: String,
    pub days_since_high: usize,
    pub low_52w: f64,
    pub low_52w_date: String,
    pub days_since_low: usize,
    pub pct_from_high: f64,           // (latest - high) / high × 100 — negative or 0
    pub pct_from_low: f64,            // (latest - low) / low × 100 — positive or 0
    pub range_position_pct: f64,      // (latest - low) / (high - low) × 100
    pub proximity_label: String,      // "AT_HIGH" | "NEAR_HIGH" | "MID_RANGE" | "NEAR_LOW" | "AT_LOW" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// RVCONE — Multi-horizon Realized Volatility Cone.
/// Pure symbol-local HP stat. Computes 20d/60d/120d/252d annualized
/// realized volatility (stdev of log returns × √252) from cached bars,
/// plus a cone-position percentile of the latest 20d RV vs the rolling
/// distribution of 20d RVs over the full window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealizedVolConeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub latest_close: f64,
    pub rv20_pct: f64,                // annualized realized vol over 20 sessions
    pub rv60_pct: f64,
    pub rv120_pct: f64,
    pub rv252_pct: f64,
    pub rv20_min_pct: f64,            // min of all rolling 20d RVs in the window
    pub rv20_median_pct: f64,         // median of rolling 20d RVs
    pub rv20_max_pct: f64,            // max of rolling 20d RVs
    pub rv20_percentile: f64,         // latest 20d RV percentile vs rolling distribution (0-100)
    pub cone_label: String,           // "COMPRESSED" | "BELOW_AVG" | "TYPICAL" | "ELEVATED" | "EXTREME" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// CALPB — Calendar Period Breakdowns.
/// Pure symbol-local HP stat that aligns to calendar boundaries rather
/// than rolling-session offsets. Emits MTD/QTD/current-year returns
/// plus prior-quarter and prior-year returns for comparison, and a
/// momentum-vs-prior-period label.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalendarPeriodBreakdownSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub latest_close: f64,
    pub mtd_pct: f64,                 // current month-to-date return
    pub qtd_pct: f64,                 // current quarter-to-date return
    pub ytd_pct: f64,                 // current year-to-date return (calendar)
    pub prior_quarter_pct: f64,       // prior calendar quarter return
    pub prior_year_pct: f64,          // prior calendar year return
    pub current_year: String,
    pub current_quarter: String,      // e.g. "Q2"
    pub momentum_label: String,       // "ACCELERATING" | "STEADY" | "DECELERATING" | "REVERSING" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── ADR-129 Round 22 — HP return-distribution + behavior stats ──

/// RETSKEW — Return distribution skewness (third standardized moment).
/// Pure symbol-local HP stat over the trailing 253-session window of log
/// returns. Positive skew → large upside outliers; negative skew → large
/// downside outliers. Complements RVCONE (second moment) and RETKURT
/// (fourth moment) with a third-moment tail-asymmetry view.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReturnSkewnessSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,              // number of log returns used
    pub mean_log_return: f64,
    pub stdev_log_return: f64,
    pub skewness: f64,                 // third standardized moment
    pub positive_return_pct: f64,      // share of up-days
    pub largest_up_pct: f64,           // max log-return (×100)
    pub largest_down_pct: f64,         // min log-return (×100)
    pub skew_label: String,            // "STRONG_LEFT" | "LEFT" | "SYMMETRIC" | "RIGHT" | "STRONG_RIGHT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// RETKURT — Return distribution excess kurtosis (fourth standardized moment - 3).
/// Pure symbol-local HP stat over the trailing 253-session window of log
/// returns. High excess kurtosis → fat-tailed distribution with more
/// extreme moves than a normal would predict.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReturnKurtosisSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mean_log_return: f64,
    pub stdev_log_return: f64,
    pub excess_kurtosis: f64,          // fourth standardized moment - 3
    pub outlier_2sigma_count: usize,   // count of |z| > 2 returns
    pub outlier_3sigma_count: usize,   // count of |z| > 3 returns
    pub outlier_2sigma_pct: f64,       // share of |z| > 2 returns (normal ≈ 4.55%)
    pub kurt_label: String,            // "PLATYKURTIC" | "NORMAL" | "MILD_FAT" | "FAT" | "EXTREME_FAT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// TAILR — Tail ratio = 95th pct return / |5th pct return|.
/// Pure symbol-local HP stat. Ratio > 1 → upside tail dominates;
/// < 1 → downside tail dominates. Complements RETSKEW with a
/// non-parametric quantile-based view of tail asymmetry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TailRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pct_95_return: f64,            // 95th percentile return (as %)
    pub pct_05_return: f64,            // 5th percentile return (as %)
    pub pct_99_return: f64,            // 99th percentile return
    pub pct_01_return: f64,            // 1st percentile return
    pub tail_ratio: f64,               // pct_95 / |pct_05|
    pub tail_ratio_99_01: f64,         // pct_99 / |pct_01|
    pub bias_label: String,            // "DOWNSIDE_HEAVY" | "SLIGHT_DOWNSIDE" | "BALANCED" | "SLIGHT_UPSIDE" | "UPSIDE_HEAVY" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// RUNLEN — Up/down day run length statistics.
/// Pure symbol-local HP stat. Average and longest runs of consecutive
/// up-days and down-days over the trailing 253-session window. Long
/// runs → trending regime; short runs → choppy / mean-reverting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunLengthSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub avg_up_run: f64,
    pub avg_down_run: f64,
    pub longest_up_run: usize,
    pub longest_down_run: usize,
    pub up_runs_count: usize,
    pub down_runs_count: usize,
    pub current_run_length: i32,       // positive = up run, negative = down run, 0 = flat
    pub trend_label: String,           // "CHOPPY" | "MIXED" | "TRENDING" | "STRONG_TRENDING" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// DAYRANGE — Daily range analysis.
/// Pure symbol-local HP stat. Average (high - low) / close over 60
/// sessions vs 252-session baseline. Ratio < 1 → compressed (expect
/// breakout); > 1 → expanded (volatility regime).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyRangeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub avg_range_60_pct: f64,         // avg (high-low)/close × 100 over 60d
    pub avg_range_252_pct: f64,        // avg (high-low)/close × 100 over 252d
    pub latest_range_pct: f64,         // latest bar's (high-low)/close × 100
    pub compression_ratio: f64,        // 60d avg / 252d avg (1.0 = neutral)
    pub widest_range_pct: f64,         // max (high-low)/close × 100 in window
    pub narrowest_range_pct: f64,      // min (high-low)/close × 100 in window
    pub range_label: String,           // "TIGHT" | "COMPRESSED" | "NORMAL" | "EXPANDED" | "VERY_EXPANDED" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── ADR-131 Godel Parity Round 23 (AUTOCOR / HURST / HITRATE / GLASYM / VOLRATIO) ──
//
// Five pure HP-local stat surfaces. All five compute over the
// trailing 253-session window of the existing `research_historical_price`
// cache and add zero new API dependencies. Each one fills a
// conceptually distinct gap vs Godel:
//
//  * AUTOCOR  — serial correlation of returns at lags 1/5/10/20,
//               the canonical momentum-vs-mean-reversion detector
//  * HURST    — long-memory exponent via rescaled-range (R/S)
//               analysis (H<0.5 mean-reverting, H≈0.5 random walk,
//               H>0.5 persistent / trending)
//  * HITRATE  — multi-horizon win rate: share of positive-return
//               bars over 5d/20d/60d/252d windows
//  * GLASYM   — gain/loss magnitude asymmetry: avg/median up-day
//               size vs avg/median down-day size + magnitude ratio
//  * VOLRATIO — accumulation/distribution hint from HP volume:
//               avg volume on up-days vs down-days + regime label

/// AUTOCOR — Autocorrelation of daily log returns at multiple lags.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Positive lag-1 ACF → momentum at the daily scale; negative → mean
/// reversion; near-zero → random-walk-like. Including longer lags
/// catches horizon-dependent regimes missed by lag-1 alone.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AutocorrelationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,              // number of log returns used
    pub lag1_acf: f64,                 // autocorrelation at lag 1
    pub lag5_acf: f64,                 // autocorrelation at lag 5
    pub lag10_acf: f64,                // autocorrelation at lag 10
    pub lag20_acf: f64,                // autocorrelation at lag 20
    pub mean_log_return: f64,
    pub regime_label: String,          // "MEAN_REVERTING" | "NEUTRAL" | "MOMENTUM" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// HURST — Hurst exponent via rescaled-range (R/S) analysis.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// H ∈ [0,1]: H<0.5 anti-persistent / mean-reverting,
/// H≈0.5 random walk, H>0.5 persistent / trending.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HurstSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub hurst_exponent: f64,
    pub scales_used: usize,            // number of R/S scales fit
    pub min_scale: usize,              // smallest chunk size
    pub max_scale: usize,              // largest chunk size
    pub memory_label: String,          // "STRONG_MEAN_REVERT" | "MEAN_REVERT" | "RANDOM_WALK" | "PERSISTENT" | "STRONG_PERSISTENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// HITRATE — Multi-horizon hit rate.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Fraction of positive-return bars at 5d / 20d / 60d / 252d sliding
/// windows. Also reports the all-window share for context. Bullish
/// when every short-horizon window is above 55%; bearish when every
/// short-horizon window is below 45%.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HitRateSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub hitrate_5d: f64,               // positive share over last 5 bars
    pub hitrate_20d: f64,              // positive share over last 20 bars
    pub hitrate_60d: f64,              // positive share over last 60 bars
    pub hitrate_252d: f64,             // positive share over last 252 bars
    pub up_days: usize,
    pub down_days: usize,
    pub flat_days: usize,
    pub hit_label: String,             // "BEARISH" | "WEAK_BEARISH" | "NEUTRAL" | "WEAK_BULLISH" | "BULLISH" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// GLASYM — Gain/loss asymmetry.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Compares the magnitude of up-days vs down-days, independent of
/// count. Ratio > 1 → typical up-day bigger than typical down-day
/// (upside asymmetry); < 1 → downside asymmetry. Complements RETSKEW
/// (third-moment tail asymmetry) with an average-move view.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GainLossAsymmetrySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub avg_up_pct: f64,               // mean |up-day return| (%)
    pub avg_down_pct: f64,             // mean |down-day return| (%)
    pub median_up_pct: f64,            // median |up-day return| (%)
    pub median_down_pct: f64,          // median |down-day return| (%)
    pub magnitude_ratio: f64,          // avg_up_pct / avg_down_pct
    pub up_days: usize,
    pub down_days: usize,
    pub asymmetry_label: String,       // "DOWNSIDE_HEAVY" | "SLIGHT_DOWNSIDE" | "BALANCED" | "SLIGHT_UPSIDE" | "UPSIDE_HEAVY" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// VOLRATIO — Up-day volume vs down-day volume ratio.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Ratio > 1 → heavier volume on up-days than down-days
/// (accumulation); < 1 → heavier volume on down-days (distribution).
/// Uses the `volume` field of HP bars so it gracefully emits
/// INSUFFICIENT_DATA when the cache was populated without volume.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolumeRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub avg_up_volume: f64,            // mean volume on up-days
    pub avg_down_volume: f64,          // mean volume on down-days
    pub median_up_volume: f64,
    pub median_down_volume: f64,
    pub up_down_volume_ratio: f64,     // avg_up_volume / avg_down_volume
    pub max_up_volume: f64,            // largest single up-day volume in window
    pub max_down_volume: f64,          // largest single down-day volume in window
    pub up_days: usize,
    pub down_days: usize,
    pub flow_label: String,            // "DISTRIBUTION" | "SLIGHT_DISTRIBUTION" | "NEUTRAL" | "SLIGHT_ACCUMULATION" | "ACCUMULATION" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── ADR-132 Round 24 — HP drawup/gap/vol-cluster/close-placement/AR(1) stats ──

/// DRAWUP — Rally history (mirror of DDHIST).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Tracks the running trough and each run from trough-to-peak: max
/// drawup, longest duration, and count of ≥5% / ≥10% rallies.
/// Complements DDHIST (ADR-127) with the upside equivalent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawupHistorySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub max_drawup_pct: f64,           // deepest rally from a trough (positive)
    pub max_drawup_trough_date: String,
    pub max_drawup_peak_date: String,
    pub longest_drawup_days: usize,    // sessions from trough to next failure or end of window
    pub rallies_5pct: usize,           // count of local-trough-to-peak advances ≥5%
    pub rallies_10pct: usize,          // count of local-trough-to-peak advances ≥10%
    pub current_drawup_pct: f64,       // latest close vs running trough (positive or 0)
    pub rally_label: String,           // "MUTED" | "MILD" | "MEANINGFUL" | "STRONG" | "EXPLOSIVE" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// GAPSTATS — Overnight gap statistics.
/// Pure symbol-local HP stat. A "gap" is `(open_t - close_{t-1}) / close_{t-1}`.
/// Reports gap frequency and magnitude in both directions plus the single
/// largest gap up / down in the window. First surface in the packet to
/// read the bar.open field rather than close-only. Label classifies the
/// bias as UP_BIAS / NEUTRAL / DOWN_BIAS based on the average net gap.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GapStatsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub gap_up_count: usize,           // gap > +0.5%
    pub gap_down_count: usize,         // gap < -0.5%
    pub avg_gap_pct: f64,              // mean of all gap %s
    pub avg_gap_up_pct: f64,           // mean of up-gaps only
    pub avg_gap_down_pct: f64,         // mean of down-gaps only
    pub largest_gap_up_pct: f64,       // single largest gap up
    pub largest_gap_down_pct: f64,     // single largest gap down (negative)
    pub gap_frequency_pct: f64,        // (gap_up + gap_down) / total_bars * 100
    pub bias_label: String,            // "DOWN_BIAS" | "SLIGHT_DOWN" | "NEUTRAL" | "SLIGHT_UP" | "UP_BIAS" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// VOLCLUSTER — Volatility clustering autocorrelation.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// ACF of squared returns and |returns| — the canonical test for ARCH /
/// GARCH effects. High |r| autocorrelation at lag 1 means "big moves
/// follow big moves" (volatility clustering) even if AUTOCOR shows no
/// serial dependence in return sign. Label is bucketed from lag-1 ACF
/// of absolute returns because that's the most common reference metric.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolClusterSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub sq_acf_lag1: f64,              // ACF of r² at lag 1
    pub sq_acf_lag5: f64,
    pub sq_acf_lag20: f64,
    pub abs_acf_lag1: f64,             // ACF of |r| at lag 1
    pub abs_acf_lag5: f64,
    pub abs_acf_lag20: f64,
    pub cluster_label: String,         // "NONE" | "MILD" | "MODERATE" | "STRONG" | "VERY_STRONG" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// CLOSEPLC — Close placement within daily range.
/// Pure symbol-local HP stat. For each bar: `pos = (close - low) / (high - low)`
/// (∈ [0, 1]). Averaged over the window, this captures bar "anatomy":
/// near 1.0 → closes typically pin near the high (buyers in control),
/// near 0.0 → closes near the low (sellers in control). Reports the
/// share of bars that closed in the top 20% of the range ("near high")
/// and bottom 20% ("near low") alongside the mean and median positions.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClosePlacementSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,              // bars with high > low
    pub avg_placement: f64,            // mean pos ∈ [0, 1]
    pub median_placement: f64,         // median pos ∈ [0, 1]
    pub latest_placement: f64,         // latest bar's pos
    pub pct_near_high: f64,            // % of bars with pos > 0.8
    pub pct_near_low: f64,             // % of bars with pos < 0.2
    pub placement_label: String,       // "STRONG_BEAR" | "BEAR" | "NEUTRAL" | "BULL" | "STRONG_BULL" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// MRHL — Mean-reversion half-life via AR(1) fit.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Fits `r_t = α + β r_{t-1} + ε` to log returns, then reports
/// half-life = -ln(2) / ln(|β|) for 0 < β < 1 (persistent regime with
/// finite memory decay). β ≤ 0 → same-period mean reversion (label
/// FAST_REVERT, half-life undefined). β ≥ 1 → explosive (shouldn't
/// happen on stationary log returns). Complements AUTOCOR (lag ACF)
/// and HURST (multi-scale persistence) with the explicit "how many
/// days until a shock decays" view.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MeanReversionHalfLifeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub beta: f64,                     // AR(1) slope
    pub alpha: f64,                    // AR(1) intercept
    pub half_life_days: f64,           // -ln(2) / ln(|β|) for β ∈ (0, 1); else 0
    pub r_squared: f64,                // goodness-of-fit
    pub regime_label: String,          // "FAST_REVERT" | "MEAN_REVERTING" | "NEUTRAL" | "PERSISTENT" | "STRONG_PERSISTENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── ADR-133 Round 25 — HP downside-vol / Sharpe / efficiency / wick / vol-of-vol ──
//
// Five more pure symbol-local HP surfaces computed from the trailing 253-
// session window. DOWNVOL and SHARPR are classical return-distribution risk
// metrics that AUTOCOR/HURST/GLASYM don't cover. EFFRATIO is Kaufman's
// efficiency ratio — a clean "trend vs noise" signal complementary to HURST.
// WICKBIAS pairs with CLOSEPLC on the bar-anatomy axis (wicks instead of
// body placement). VOLOFVOL captures "is the vol regime stable?" and is
// the textbook companion to VOLCLUSTER.

/// DOWNVOL — Downside deviation + Sortino ratio.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Semi-deviation uses only negative log returns: `sqrt(mean(min(r,0)²))`.
/// Sortino = `mean(r) / downside_dev` (dimensionless, same sign as mean
/// return). Complements the full-stdev view in RSTATS by isolating
/// "scary vol" from total vol. Label classifies Sortino into the
/// standard VERY_POOR / POOR / NEUTRAL / GOOD / EXCELLENT bands.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DownsideVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mean_log_return: f64,          // mean r over window
    pub downside_dev: f64,              // sqrt(mean(min(r,0)²))
    pub downside_dev_ann: f64,          // downside_dev × √252
    pub upside_dev: f64,                // sqrt(mean(max(r,0)²))
    pub sortino_ratio: f64,             // mean(r) / downside_dev
    pub sortino_ratio_ann: f64,         // (mean × 252) / downside_dev_ann
    pub downside_pct_of_total: f64,     // downside_dev² / total_var × 100
    pub sortino_label: String,          // "VERY_POOR" | "POOR" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// SHARPR — Sharpe ratio snapshot.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Classical Sharpe = `(mean_return - rf) / stdev_return`. We use `rf = 0`
/// because the HP cache doesn't carry a risk-free series and most
/// single-stock Sharpe conversations use the excess-above-zero
/// formulation. Both raw and annualized forms are reported. Label
/// classifies into POOR / BELOW_AVG / NEUTRAL / GOOD / EXCELLENT per
/// the standard buckets on annualized Sharpe.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SharpeRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mean_log_return: f64,
    pub stdev_log_return: f64,
    pub sharpe_ratio: f64,              // raw daily
    pub sharpe_ratio_ann: f64,          // × √252
    pub mean_return_ann: f64,           // mean × 252
    pub stdev_return_ann: f64,          // stdev × √252
    pub sharpe_label: String,           // "POOR" | "BELOW_AVG" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// EFFRATIO — Kaufman's efficiency ratio.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `ER = |close_N - close_1| / Σ |close_t - close_{t-1}|`. Measures
/// the "directness" of a price move — 1.0 → straight line, 0.0 → all
/// chop with zero net movement. Complements HURST (multi-scale
/// persistence) and MRHL (shock decay) with a cleaner single-number
/// "signal-to-noise in price travel" view. Label: CHOP (<0.1) /
/// NOISY (<0.25) / MIXED (<0.4) / TRENDING (<0.6) / STRONG_TREND (≥0.6).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EfficiencyRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub start_close: f64,
    pub end_close: f64,
    pub net_change: f64,                // signed end - start
    pub net_change_pct: f64,            // (end/start - 1) × 100
    pub sum_abs_changes: f64,           // Σ |close_t - close_{t-1}|
    pub efficiency_ratio: f64,          // |net| / sum_abs (signed direction separate)
    pub signed_efficiency: f64,         // efficiency_ratio × sign(net_change)
    pub efficiency_label: String,       // "CHOP" | "NOISY" | "MIXED" | "TRENDING" | "STRONG_TREND" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// WICKBIAS — Upper vs lower wick asymmetry.
/// Pure symbol-local HP stat. For each bar with `high > low`:
/// `upper_wick = (high - max(open, close)) / (high - low)`
/// `lower_wick = (min(open, close) - low) / (high - low)`.
/// Averaged over the window, this captures who rejected price at the
/// extremes: long upper wicks = sellers rejecting the high; long
/// lower wicks = buyers defending the low. Reports means, medians,
/// and a bias score = `avg_lower - avg_upper` (positive = buyers).
/// Complements CLOSEPLC (where the bar closes within its range) on
/// the wick side (how far the bar traveled outside its body).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WickBiasSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,               // bars with high > low
    pub avg_upper_wick: f64,            // mean upper wick share
    pub avg_lower_wick: f64,            // mean lower wick share
    pub median_upper_wick: f64,
    pub median_lower_wick: f64,
    pub avg_body_share: f64,            // 1 - upper - lower
    pub wick_bias_score: f64,           // avg_lower - avg_upper
    pub bias_label: String,             // "SELLER_REJECT" | "SELLER_LEAN" | "NEUTRAL" | "BUYER_LEAN" | "BUYER_DEFEND" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// VOLOFVOL — Standard deviation of rolling 20-day realized volatility.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// For each trailing window of 20 bars, compute realized vol = stdev of
/// log returns; then report mean and stdev of the resulting series. This
/// captures "is the vol regime stable, or does vol itself bounce?" —
/// a name with high vol-of-vol has unpredictable risk even if its
/// average vol is moderate. Label classifies `stdev(rv20) / mean(rv20)`
/// (coefficient of variation) into STABLE / MILD / MODERATE /
/// UNSTABLE / CHAOTIC buckets.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolOfVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,               // bars with valid rv20 values
    pub mean_rv20: f64,                 // mean of rolling 20d vol (daily)
    pub stdev_rv20: f64,                // stdev of rolling 20d vol
    pub min_rv20: f64,
    pub max_rv20: f64,
    pub latest_rv20: f64,
    pub cv_rv20: f64,                   // stdev_rv20 / mean_rv20 (coefficient of variation)
    pub cv_label: String,               // "STABLE" | "MILD" | "MODERATE" | "UNSTABLE" | "CHAOTIC" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Finnhub fetchers ───────────────────────────────────────────────────────

/// Finnhub /stock/profile2 — company profile.
pub async fn fetch_finnhub_profile(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<CompanyProfile, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/profile2")
        .query(&[("symbol", symbol), ("token", token)])
        .send().await
        .map_err(|e| format!("Finnhub profile failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub profile: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Finnhub profile parse: {e}"))?;
    Ok(CompanyProfile {
        symbol: symbol.to_uppercase(),
        name: v["name"].as_str().unwrap_or("").to_string(),
        exchange: v["exchange"].as_str().unwrap_or("").to_string(),
        country: v["country"].as_str().unwrap_or("").to_string(),
        currency: v["currency"].as_str().unwrap_or("").to_string(),
        industry: v["finnhubIndustry"].as_str().unwrap_or("").to_string(),
        sector: v["gind"].as_str().unwrap_or("").to_string(),
        website: v["weburl"].as_str().unwrap_or("").to_string(),
        logo: v["logo"].as_str().unwrap_or("").to_string(),
        phone: v["phone"].as_str().unwrap_or("").to_string(),
        ipo_date: v["ipo"].as_str().unwrap_or("").to_string(),
        market_cap: v["marketCapitalization"].as_f64().unwrap_or(0.0),
        shares_outstanding: v["shareOutstanding"].as_f64().unwrap_or(0.0),
    })
}

/// Finnhub /stock/peers — related tickers (up to ~10).
pub async fn fetch_finnhub_peers(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<String>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/peers")
        .query(&[("symbol", symbol), ("token", token)])
        .send().await
        .map_err(|e| format!("Finnhub peers failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub peers: HTTP {}", resp.status()));
    }
    let arr: Vec<String> = resp.json().await
        .map_err(|e| format!("Finnhub peers parse: {e}"))?;
    Ok(arr)
}

/// Finnhub /stock/earnings — actual vs estimate EPS per quarter (up to ~16 rows).
pub async fn fetch_finnhub_earnings(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<EarningRow>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/earnings")
        .query(&[("symbol", symbol), ("token", token)])
        .send().await
        .map_err(|e| format!("Finnhub earnings failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub earnings: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("Finnhub earnings parse: {e}"))?;
    let rows = arr.into_iter().map(|e| {
        let actual = e["actual"].as_f64();
        let estimate = e["estimate"].as_f64();
        let surprise = e["surprise"].as_f64();
        let surprise_pct = e["surprisePercent"].as_f64();
        EarningRow {
            period: e["period"].as_str().unwrap_or("").to_string(),
            actual, estimate, surprise, surprise_pct,
            quarter: e["quarter"].as_i64().map(|v| v as i32),
            year: e["year"].as_i64().map(|v| v as i32),
        }
    }).collect();
    Ok(rows)
}

/// Finnhub /calendar/ipo — upcoming IPOs in a date range.
pub async fn fetch_finnhub_ipo_calendar(
    client: &reqwest::Client,
    token: &str,
    from: &str,
    to: &str,
) -> Result<Vec<IpoEvent>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/calendar/ipo")
        .query(&[("token", token), ("from", from), ("to", to)])
        .send().await
        .map_err(|e| format!("Finnhub IPO calendar failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub IPO: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Finnhub IPO parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["ipoCalendar"].as_array() {
        for e in arr {
            rows.push(IpoEvent {
                date: e["date"].as_str().unwrap_or("").to_string(),
                symbol: e["symbol"].as_str().unwrap_or("").to_string(),
                name: e["name"].as_str().unwrap_or("").to_string(),
                exchange: e["exchange"].as_str().unwrap_or("").to_string(),
                price_range: e["price"].as_str().unwrap_or("").to_string(),
                shares: e["numberOfShares"].as_i64().unwrap_or(0),
                total_value: e["totalSharesValue"].as_f64().unwrap_or(0.0),
                status: e["status"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    Ok(rows)
}

/// Finnhub /press-releases — company press releases (last 90 days).
pub async fn fetch_finnhub_press(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<PressRelease>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let ninety_ago = (chrono::Utc::now() - chrono::Duration::days(90)).format("%Y-%m-%d").to_string();
    let resp = client
        .get("https://finnhub.io/api/v1/press-releases")
        .query(&[("symbol", symbol), ("token", token), ("from", ninety_ago.as_str()), ("to", today.as_str())])
        .send().await
        .map_err(|e| format!("Finnhub press failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub press: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Finnhub press parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["majorDevelopment"].as_array() {
        for e in arr {
            rows.push(PressRelease {
                symbol: symbol.to_uppercase(),
                datetime: e["datetime"].as_str().unwrap_or("").to_string(),
                headline: e["headline"].as_str().unwrap_or("").to_string(),
                description: e["description"].as_str().unwrap_or("").to_string(),
                url: e["url"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    Ok(rows)
}

/// Finnhub /stock/social-sentiment — Reddit + Twitter daily mention buckets (last 30 days).
pub async fn fetch_finnhub_social(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<SocialSentimentRow>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let month_ago = (chrono::Utc::now() - chrono::Duration::days(30)).format("%Y-%m-%d").to_string();
    let resp = client
        .get("https://finnhub.io/api/v1/stock/social-sentiment")
        .query(&[("symbol", symbol), ("token", token), ("from", month_ago.as_str()), ("to", today.as_str())])
        .send().await
        .map_err(|e| format!("Finnhub social failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub social: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Finnhub social parse: {e}"))?;
    let mut rows = Vec::new();
    for src in ["reddit", "twitter"].iter() {
        if let Some(arr) = v[src].as_array() {
            for e in arr {
                rows.push(SocialSentimentRow {
                    source: src.to_string(),
                    at_time: e["atTime"].as_str().unwrap_or("").to_string(),
                    mention: e["mention"].as_i64().unwrap_or(0),
                    positive_mention: e["positiveMention"].as_i64().unwrap_or(0),
                    negative_mention: e["negativeMention"].as_i64().unwrap_or(0),
                    positive_score: e["positiveScore"].as_f64().unwrap_or(0.0),
                    negative_score: e["negativeScore"].as_f64().unwrap_or(0.0),
                    score: e["score"].as_f64().unwrap_or(0.0),
                });
            }
        }
    }
    Ok(rows)
}

// ── FMP fetchers ───────────────────────────────────────────────────────────

/// FMP /earning_call_transcript/{symbol} list endpoint — returns available [year, quarter, date] triples.
pub async fn fetch_fmp_transcript_list(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<TranscriptMeta>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    // FMP returns e.g. [[4, 2023, "2024-02-01"], [3, 2023, "2023-11-02"], ...]
    let url = format!("https://financialmodelingprep.com/api/v4/earning_call_transcript?symbol={}&apikey={}", symbol, fmp_key);
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP transcript list failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP transcript list: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("FMP transcript list parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v.as_array() {
        for entry in arr {
            if let Some(triple) = entry.as_array() {
                if triple.len() >= 3 {
                    let quarter = triple[0].as_i64().unwrap_or(0) as i32;
                    let year = triple[1].as_i64().unwrap_or(0) as i32;
                    let date = triple[2].as_str().unwrap_or("").to_string();
                    rows.push(TranscriptMeta {
                        symbol: symbol.to_uppercase(),
                        quarter, year, date,
                    });
                }
            }
        }
    }
    Ok(rows)
}

/// FMP /earning_call_transcript/{symbol}?quarter=N&year=Y — full transcript body.
pub async fn fetch_fmp_transcript(
    client: &reqwest::Client,
    symbol: &str,
    quarter: i32,
    year: i32,
    fmp_key: &str,
) -> Result<Transcript, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!("https://financialmodelingprep.com/api/v3/earning_call_transcript/{}?quarter={}&year={}&apikey={}",
        symbol, quarter, year, fmp_key);
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP transcript failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP transcript: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP transcript parse: {e}"))?;
    if arr.is_empty() {
        return Err(format!("No transcript for {} Q{} {}", symbol, quarter, year));
    }
    let e = &arr[0];
    Ok(Transcript {
        symbol: symbol.to_uppercase(),
        quarter: e["quarter"].as_i64().unwrap_or(quarter as i64) as i32,
        year: e["year"].as_i64().unwrap_or(year as i64) as i32,
        date: e["date"].as_str().unwrap_or("").to_string(),
        content: e["content"].as_str().unwrap_or("").to_string(),
    })
}

// ── Yahoo fetchers ─────────────────────────────────────────────────────────

/// Yahoo /v7/finance/quote — batch commodities quote.
/// Returns (symbol, display_name, price, change, change_pct).
pub async fn fetch_yahoo_quotes(
    client: &reqwest::Client,
    symbols: &[&str],
) -> Result<Vec<(String, f64, f64, f64)>, String> {
    if symbols.is_empty() { return Ok(vec![]); }
    let joined = symbols.join(",");
    let url = format!("https://query1.finance.yahoo.com/v7/finance/quote?symbols={}", joined);
    let resp = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
        .send().await
        .map_err(|e| format!("Yahoo quote failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Yahoo quote: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Yahoo quote parse: {e}"))?;
    let mut out = Vec::new();
    if let Some(arr) = v.pointer("/quoteResponse/result").and_then(|r| r.as_array()) {
        for q in arr {
            let sym = q["symbol"].as_str().unwrap_or("").to_string();
            let price = q["regularMarketPrice"].as_f64().unwrap_or(0.0);
            let change = q["regularMarketChange"].as_f64().unwrap_or(0.0);
            let pct = q["regularMarketChangePercent"].as_f64().unwrap_or(0.0);
            if !sym.is_empty() {
                out.push((sym, price, change, pct));
            }
        }
    }
    Ok(out)
}

// ── SQLite cache schema ────────────────────────────────────────────────────

/// Create the research_* cache tables on the given connection (idempotent).
pub fn create_research_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_profile (
            symbol TEXT PRIMARY KEY,
            name TEXT NOT NULL DEFAULT '',
            exchange TEXT NOT NULL DEFAULT '',
            country TEXT NOT NULL DEFAULT '',
            currency TEXT NOT NULL DEFAULT '',
            industry TEXT NOT NULL DEFAULT '',
            sector TEXT NOT NULL DEFAULT '',
            website TEXT NOT NULL DEFAULT '',
            logo TEXT NOT NULL DEFAULT '',
            phone TEXT NOT NULL DEFAULT '',
            ipo_date TEXT NOT NULL DEFAULT '',
            market_cap REAL NOT NULL DEFAULT 0,
            shares_outstanding REAL NOT NULL DEFAULT 0,
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_peers (
            symbol TEXT PRIMARY KEY,
            peers_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_earnings (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_press (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_sentiment (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_transcript_list (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_transcript (
            symbol TEXT NOT NULL,
            quarter INTEGER NOT NULL,
            year INTEGER NOT NULL,
            date TEXT NOT NULL DEFAULT '',
            content TEXT NOT NULL DEFAULT '',
            updated_at INTEGER NOT NULL DEFAULT 0,
            PRIMARY KEY (symbol, year, quarter)
        );
        CREATE TABLE IF NOT EXISTS research_ipo_calendar (
            snapshot_at INTEGER PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]'
        );
        CREATE INDEX IF NOT EXISTS idx_research_profile_updated ON research_profile(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_peers_updated ON research_peers(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_earnings_updated ON research_earnings(updated_at);"
    ).map_err(|e| format!("create research tables: {e}"))?;
    Ok(())
}

fn now_ts() -> i64 { chrono::Utc::now().timestamp() }

// ── profile ────────────────────────────────────────────────────────────────

pub fn upsert_profile(conn: &Connection, p: &CompanyProfile) -> Result<(), String> {
    let _ = create_research_tables(conn);
    conn.execute(
        "INSERT INTO research_profile
         (symbol, name, exchange, country, currency, industry, sector, website, logo, phone, ipo_date, market_cap, shares_outstanding, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
         ON CONFLICT(symbol) DO UPDATE SET
            name=excluded.name, exchange=excluded.exchange, country=excluded.country,
            currency=excluded.currency, industry=excluded.industry, sector=excluded.sector,
            website=excluded.website, logo=excluded.logo, phone=excluded.phone,
            ipo_date=excluded.ipo_date, market_cap=excluded.market_cap,
            shares_outstanding=excluded.shares_outstanding, updated_at=excluded.updated_at",
        params![
            p.symbol.to_uppercase(), p.name, p.exchange, p.country, p.currency,
            p.industry, p.sector, p.website, p.logo, p.phone, p.ipo_date,
            p.market_cap, p.shares_outstanding, now_ts(),
        ],
    ).map_err(|e| format!("upsert profile: {e}"))?;
    Ok(())
}

pub fn get_profile(conn: &Connection, symbol: &str) -> Result<Option<CompanyProfile>, String> {
    let _ = create_research_tables(conn);
    let sym = symbol.to_uppercase();
    let mut stmt = conn.prepare(
        "SELECT symbol, name, exchange, country, currency, industry, sector, website, logo, phone, ipo_date, market_cap, shares_outstanding
         FROM research_profile WHERE symbol = ?1"
    ).map_err(|e| format!("prepare get_profile: {e}"))?;
    let mut rows = stmt.query(params![sym]).map_err(|e| format!("query get_profile: {e}"))?;
    if let Some(row) = rows.next().map_err(|e| format!("row get_profile: {e}"))? {
        Ok(Some(CompanyProfile {
            symbol: row.get(0).unwrap_or_default(),
            name: row.get(1).unwrap_or_default(),
            exchange: row.get(2).unwrap_or_default(),
            country: row.get(3).unwrap_or_default(),
            currency: row.get(4).unwrap_or_default(),
            industry: row.get(5).unwrap_or_default(),
            sector: row.get(6).unwrap_or_default(),
            website: row.get(7).unwrap_or_default(),
            logo: row.get(8).unwrap_or_default(),
            phone: row.get(9).unwrap_or_default(),
            ipo_date: row.get(10).unwrap_or_default(),
            market_cap: row.get(11).unwrap_or(0.0),
            shares_outstanding: row.get(12).unwrap_or(0.0),
        }))
    } else {
        Ok(None)
    }
}

// ── peers ──────────────────────────────────────────────────────────────────

pub fn upsert_peers(conn: &Connection, symbol: &str, peers: &[String]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(peers).map_err(|e| format!("peers json: {e}"))?;
    conn.execute(
        "INSERT INTO research_peers(symbol, peers_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET peers_json=excluded.peers_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert peers: {e}"))?;
    Ok(())
}

pub fn get_peers(conn: &Connection, symbol: &str) -> Result<Option<Vec<String>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare("SELECT peers_json FROM research_peers WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_peers: {e}"))?;
    let mut rows = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_peers: {e}"))?;
    if let Some(row) = rows.next().map_err(|e| format!("row get_peers: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        let peers: Vec<String> = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(peers))
    } else {
        Ok(None)
    }
}

// ── earnings history ───────────────────────────────────────────────────────

pub fn upsert_earnings_history(conn: &Connection, symbol: &str, rows: &[EarningRow]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("earnings json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earnings(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert earnings: {e}"))?;
    Ok(())
}

pub fn get_earnings_history(conn: &Connection, symbol: &str) -> Result<Option<Vec<EarningRow>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_earnings WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_earnings: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_earnings: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_earnings: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        let rows: Vec<EarningRow> = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(rows))
    } else {
        Ok(None)
    }
}

// ── press releases ─────────────────────────────────────────────────────────

pub fn upsert_press_releases(conn: &Connection, symbol: &str, rows: &[PressRelease]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("press json: {e}"))?;
    conn.execute(
        "INSERT INTO research_press(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert press: {e}"))?;
    Ok(())
}

pub fn get_press_releases(conn: &Connection, symbol: &str) -> Result<Option<Vec<PressRelease>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_press WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_press: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_press: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_press: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        let rows: Vec<PressRelease> = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(rows))
    } else {
        Ok(None)
    }
}

// ── social sentiment ───────────────────────────────────────────────────────

pub fn upsert_sentiment(conn: &Connection, symbol: &str, rows: &[SocialSentimentRow]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("sentiment json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sentiment(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sentiment: {e}"))?;
    Ok(())
}

pub fn get_sentiment(conn: &Connection, symbol: &str) -> Result<Option<Vec<SocialSentimentRow>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_sentiment WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sentiment: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_sentiment: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sentiment: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        let rows: Vec<SocialSentimentRow> = serde_json::from_str(&json).unwrap_or_default();
        Ok(Some(rows))
    } else {
        Ok(None)
    }
}

// ── transcripts ────────────────────────────────────────────────────────────

pub fn upsert_transcript_list(conn: &Connection, symbol: &str, rows: &[TranscriptMeta]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("transcript list json: {e}"))?;
    conn.execute(
        "INSERT INTO research_transcript_list(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert transcript list: {e}"))?;
    Ok(())
}

pub fn get_transcript_list(conn: &Connection, symbol: &str) -> Result<Option<Vec<TranscriptMeta>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_transcript_list WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_tlist: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_tlist: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_tlist: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_transcript(conn: &Connection, t: &Transcript) -> Result<(), String> {
    let _ = create_research_tables(conn);
    conn.execute(
        "INSERT INTO research_transcript(symbol, quarter, year, date, content, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6)
         ON CONFLICT(symbol, year, quarter) DO UPDATE SET
            date=excluded.date, content=excluded.content, updated_at=excluded.updated_at",
        params![t.symbol.to_uppercase(), t.quarter, t.year, t.date, t.content, now_ts()],
    ).map_err(|e| format!("upsert transcript: {e}"))?;
    Ok(())
}

pub fn get_transcript(conn: &Connection, symbol: &str, quarter: i32, year: i32) -> Result<Option<Transcript>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare(
        "SELECT symbol, quarter, year, date, content FROM research_transcript
         WHERE symbol = ?1 AND year = ?2 AND quarter = ?3"
    ).map_err(|e| format!("prepare get_transcript: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase(), year, quarter])
        .map_err(|e| format!("query get_transcript: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_transcript: {e}"))? {
        Ok(Some(Transcript {
            symbol: row.get(0).unwrap_or_default(),
            quarter: row.get(1).unwrap_or(0),
            year: row.get(2).unwrap_or(0),
            date: row.get(3).unwrap_or_default(),
            content: row.get(4).unwrap_or_default(),
        }))
    } else {
        Ok(None)
    }
}

// ── IPO calendar ───────────────────────────────────────────────────────────

pub fn upsert_ipo_calendar(conn: &Connection, rows: &[IpoEvent]) -> Result<(), String> {
    let _ = create_research_tables(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("ipo json: {e}"))?;
    conn.execute("DELETE FROM research_ipo_calendar", []).map_err(|e| format!("ipo delete: {e}"))?;
    conn.execute(
        "INSERT INTO research_ipo_calendar(snapshot_at, rows_json) VALUES (?1,?2)",
        params![now_ts(), json],
    ).map_err(|e| format!("upsert ipo: {e}"))?;
    Ok(())
}

pub fn get_ipo_calendar(conn: &Connection) -> Result<Option<Vec<IpoEvent>>, String> {
    let _ = create_research_tables(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_ipo_calendar ORDER BY snapshot_at DESC LIMIT 1")
        .map_err(|e| format!("prepare get_ipo: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_ipo: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ipo: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── bulk scrape helper (used by fundamentals scrape loop) ──────────────────

/// Fetch and cache all research data for a single symbol, respecting rate limits.
/// Returns Ok(()) even if individual endpoints fail — errors are logged via cb.
pub async fn scrape_and_cache_symbol(
    client: &reqwest::Client,
    conn: &Connection,
    symbol: &str,
    finnhub_key: &str,
    fmp_key: &str,
    mut cb: impl FnMut(&str),
) -> Result<(), String> {
    let sym = symbol.to_uppercase();
    if sym.is_empty() { return Err("empty symbol".into()); }

    // Profile
    if !finnhub_key.is_empty() {
        match fetch_finnhub_profile(client, &sym, finnhub_key).await {
            Ok(p) => {
                if !p.name.is_empty() {
                    let _ = upsert_profile(conn, &p);
                    cb(&format!("research/profile: {} cached", sym));
                }
            }
            Err(e) => cb(&format!("research/profile {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Peers
        match fetch_finnhub_peers(client, &sym, finnhub_key).await {
            Ok(peers) => {
                if !peers.is_empty() {
                    let _ = upsert_peers(conn, &sym, &peers);
                }
            }
            Err(e) => cb(&format!("research/peers {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Earnings
        match fetch_finnhub_earnings(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_earnings_history(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/earnings {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Press releases
        match fetch_finnhub_press(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_press_releases(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/press {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Social sentiment
        match fetch_finnhub_social(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_sentiment(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/sentiment {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }

    // Transcripts (FMP)
    if !fmp_key.is_empty() {
        match fetch_fmp_transcript_list(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_transcript_list(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/transcripts {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-109: dividend history (FMP)
        match fetch_fmp_dividend_history(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_dividends(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/dividends {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-109: forward earnings estimates (FMP)
        match fetch_fmp_earnings_estimates(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_earnings_estimates(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/estimates {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-109: analyst rating changes (FMP)
        match fetch_fmp_rating_changes(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_rating_changes(conn, &sym, &rows);
                }
            }
            Err(e) => cb(&format!("research/ratings {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-110: full FA bundle (6 FMP calls, internal 400ms sleeps).
        match fetch_fmp_financial_bundle(client, &sym, fmp_key).await {
            Ok(bundle) => {
                let any = !bundle.income_annual.is_empty()
                    || !bundle.income_quarterly.is_empty()
                    || !bundle.balance_annual.is_empty()
                    || !bundle.balance_quarterly.is_empty()
                    || !bundle.cashflow_annual.is_empty()
                    || !bundle.cashflow_quarterly.is_empty();
                if any {
                    let _ = upsert_financials(conn, &sym, &bundle);
                    cb(&format!("research/financials: {} cached", sym));
                }
            }
            Err(e) => cb(&format!("research/financials {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-111: stock split history (FMP).
        match fetch_fmp_stock_splits(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_stock_splits(conn, &sym, &rows);
                    cb(&format!("research/splits: {} cached ({} rows)", sym, rows.len()));
                }
            }
            Err(e) => cb(&format!("research/splits {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-111: ETF holdings (FMP). No-op for non-ETF tickers (empty result).
        match fetch_fmp_etf_holdings(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_etf_holdings(conn, &sym, &rows);
                    cb(&format!("research/etf: {} cached ({} holdings)", sym, rows.len()));
                }
            }
            Err(e) => cb(&format!("research/etf {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;

        // ADR-111: ESG scores (FMP).
        match fetch_fmp_esg(client, &sym, fmp_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_esg(conn, &sym, &rows);
                    cb(&format!("research/esg: {} cached ({} years)", sym, rows.len()));
                }
            }
            Err(e) => cb(&format!("research/esg {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    }

    // ADR-110: Finnhub executives (separate from FMP block; needs Finnhub key).
    if !finnhub_key.is_empty() {
        match fetch_finnhub_executives(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_executives(conn, &sym, &rows);
                    cb(&format!("research/executives: {} cached ({} rows)", sym, rows.len()));
                }
            }
            Err(e) => cb(&format!("research/executives {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // ADR-111: analyst recommendation trends (Finnhub).
        match fetch_finnhub_recommendations(client, &sym, finnhub_key).await {
            Ok(rows) => {
                if !rows.is_empty() {
                    let _ = upsert_analyst_recs(conn, &sym, &rows);
                    cb(&format!("research/recs: {} cached ({} rows)", sym, rows.len()));
                }
            }
            Err(e) => cb(&format!("research/recs {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // ADR-111: consensus price target (Finnhub).
        match fetch_finnhub_price_target(client, &sym, finnhub_key).await {
            Ok(pt) => {
                if pt.num_analysts > 0 || pt.target_mean > 0.0 {
                    let _ = upsert_price_target(conn, &sym, &pt);
                    cb(&format!("research/target: {} cached (n={})", sym, pt.num_analysts));
                }
            }
            Err(e) => cb(&format!("research/target {} failed: {}", sym, e)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;
    }

    Ok(())
}

// ── ADR-109 fetchers ───────────────────────────────────────────────────────

/// FMP /historical-price-full/stock_dividend/{symbol} — full dividend payment history.
pub async fn fetch_fmp_dividend_history(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<DividendRecord>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/historical-price-full/stock_dividend/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP dividends failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP dividends: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("FMP dividends parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["historical"].as_array() {
        for e in arr {
            rows.push(DividendRecord {
                ex_date: e["date"].as_str().unwrap_or("").to_string(),
                pay_date: e["paymentDate"].as_str().unwrap_or("").to_string(),
                record_date: e["recordDate"].as_str().unwrap_or("").to_string(),
                declaration_date: e["declarationDate"].as_str().unwrap_or("").to_string(),
                amount: e["dividend"].as_f64().unwrap_or(0.0),
                adjusted_amount: e["adjDividend"].as_f64().unwrap_or(0.0),
                label: e["label"].as_str().unwrap_or("").to_string(),
            });
        }
    }
    Ok(rows)
}

/// FMP /analyst-estimates/{symbol} — forward EPS and revenue consensus estimates.
pub async fn fetch_fmp_earnings_estimates(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<EarningsEstimate>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/analyst-estimates/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP estimates failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP estimates: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP estimates parse: {e}"))?;
    let rows = arr.into_iter().map(|e| EarningsEstimate {
        date: e["date"].as_str().unwrap_or("").to_string(),
        eps_avg: e["estimatedEpsAvg"].as_f64().unwrap_or(0.0),
        eps_high: e["estimatedEpsHigh"].as_f64().unwrap_or(0.0),
        eps_low: e["estimatedEpsLow"].as_f64().unwrap_or(0.0),
        revenue_avg: e["estimatedRevenueAvg"].as_f64().unwrap_or(0.0),
        revenue_high: e["estimatedRevenueHigh"].as_f64().unwrap_or(0.0),
        revenue_low: e["estimatedRevenueLow"].as_f64().unwrap_or(0.0),
        num_analysts_eps: e["numberAnalystEstimatedEps"].as_i64().unwrap_or(0) as i32,
        num_analysts_rev: e["numberAnalystsEstimatedRevenue"].as_i64().unwrap_or(0) as i32,
    }).collect();
    Ok(rows)
}

/// FMP /upgrades-downgrades (v4) — analyst rating change feed for a symbol.
pub async fn fetch_fmp_rating_changes(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<RatingChange>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v4/upgrades-downgrades?symbol={}&apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP rating changes failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP rating changes: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP rating changes parse: {e}"))?;
    let rows = arr.into_iter().map(|e| {
        let to = e["newGrade"].as_str().unwrap_or("").to_string();
        let from = e["previousGrade"].as_str().unwrap_or("").to_string();
        let action_raw = e["action"].as_str().unwrap_or("").to_lowercase();
        // FMP action strings like "hold","buy" — map to upgrade/downgrade where we can.
        let action = if action_raw.is_empty() {
            if from.is_empty() { "initiation" } else if to != from { "changed" } else { "maintain" }.to_string()
        } else { action_raw };
        RatingChange {
            date: e["publishedDate"].as_str().unwrap_or("").chars().take(10).collect(),
            symbol: e["symbol"].as_str().unwrap_or(symbol).to_uppercase(),
            company: e["gradingCompany"].as_str().unwrap_or("").to_string(),
            firm: e["gradingCompany"].as_str().unwrap_or("").to_string(),
            action,
            from_grade: from,
            to_grade: to,
            price_target: e["priceTarget"].as_f64().unwrap_or(0.0),
        }
    }).collect();
    Ok(rows)
}

/// Yahoo batch quote → Treasury yield curve snapshot (no auth).
pub async fn fetch_treasury_yields(
    client: &reqwest::Client,
) -> Result<Vec<TreasuryYield>, String> {
    let tickers: Vec<&str> = TREASURY_TENORS.iter().map(|(t, _)| *t).collect();
    let quotes = fetch_yahoo_quotes(client, &tickers).await?;
    let mut out = Vec::new();
    for (sym, price, change, pct) in quotes {
        if let Some((_, tenor)) = TREASURY_TENORS.iter().find(|(t, _)| *t == sym.as_str()) {
            out.push(TreasuryYield {
                tenor: (*tenor).to_string(),
                ticker: sym,
                yield_pct: price,
                change,
                change_pct: pct,
            });
        }
    }
    // Preserve ladder order (13W, 5Y, 10Y, 30Y).
    out.sort_by_key(|t| TREASURY_TENORS.iter().position(|(_, lbl)| *lbl == t.tenor.as_str()).unwrap_or(99));
    Ok(out)
}

// ── ADR-110 fetchers ───────────────────────────────────────────────────────

/// Parse a Socrata numeric field that arrives as either a JSON number or a string.
fn socrata_f64(v: &serde_json::Value) -> f64 {
    v.as_f64()
        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
        .unwrap_or(0.0)
}

/// FMP /income-statement/{symbol} — up to 20 historical periods. `period` = "annual" or "quarter".
pub async fn fetch_fmp_income_statement(
    client: &reqwest::Client,
    symbol: &str,
    period: &str,
    fmp_key: &str,
) -> Result<Vec<IncomeStatement>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/income-statement/{}?period={}&limit=20&apikey={}",
        symbol, period, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP income failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP income: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP income parse: {e}"))?;
    let rows = arr.into_iter().map(|e| IncomeStatement {
        date: e["date"].as_str().unwrap_or("").to_string(),
        period: e["period"].as_str().unwrap_or("").to_string(),
        revenue: e["revenue"].as_f64().unwrap_or(0.0),
        cost_of_revenue: e["costOfRevenue"].as_f64().unwrap_or(0.0),
        gross_profit: e["grossProfit"].as_f64().unwrap_or(0.0),
        research_and_development: e["researchAndDevelopmentExpenses"].as_f64().unwrap_or(0.0),
        selling_general_admin: e["sellingGeneralAndAdministrativeExpenses"].as_f64().unwrap_or(0.0),
        operating_expenses: e["operatingExpenses"].as_f64().unwrap_or(0.0),
        operating_income: e["operatingIncome"].as_f64().unwrap_or(0.0),
        interest_expense: e["interestExpense"].as_f64().unwrap_or(0.0),
        ebitda: e["ebitda"].as_f64().unwrap_or(0.0),
        income_before_tax: e["incomeBeforeTax"].as_f64().unwrap_or(0.0),
        income_tax_expense: e["incomeTaxExpense"].as_f64().unwrap_or(0.0),
        net_income: e["netIncome"].as_f64().unwrap_or(0.0),
        eps: e["eps"].as_f64().unwrap_or(0.0),
        eps_diluted: e["epsdiluted"].as_f64().unwrap_or(0.0),
        weighted_shares_out: e["weightedAverageShsOut"].as_f64().unwrap_or(0.0),
    }).collect();
    Ok(rows)
}

/// FMP /balance-sheet-statement/{symbol} — up to 20 historical periods.
pub async fn fetch_fmp_balance_sheet(
    client: &reqwest::Client,
    symbol: &str,
    period: &str,
    fmp_key: &str,
) -> Result<Vec<BalanceSheet>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/balance-sheet-statement/{}?period={}&limit=20&apikey={}",
        symbol, period, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP balance failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP balance: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP balance parse: {e}"))?;
    let rows = arr.into_iter().map(|e| BalanceSheet {
        date: e["date"].as_str().unwrap_or("").to_string(),
        period: e["period"].as_str().unwrap_or("").to_string(),
        cash_and_equiv: e["cashAndCashEquivalents"].as_f64().unwrap_or(0.0),
        short_term_investments: e["shortTermInvestments"].as_f64().unwrap_or(0.0),
        net_receivables: e["netReceivables"].as_f64().unwrap_or(0.0),
        inventory: e["inventory"].as_f64().unwrap_or(0.0),
        total_current_assets: e["totalCurrentAssets"].as_f64().unwrap_or(0.0),
        property_plant_equipment: e["propertyPlantEquipmentNet"].as_f64().unwrap_or(0.0),
        goodwill: e["goodwill"].as_f64().unwrap_or(0.0),
        intangible_assets: e["intangibleAssets"].as_f64().unwrap_or(0.0),
        long_term_investments: e["longTermInvestments"].as_f64().unwrap_or(0.0),
        total_non_current_assets: e["totalNonCurrentAssets"].as_f64().unwrap_or(0.0),
        total_assets: e["totalAssets"].as_f64().unwrap_or(0.0),
        accounts_payable: e["accountPayables"].as_f64().unwrap_or(0.0),
        short_term_debt: e["shortTermDebt"].as_f64().unwrap_or(0.0),
        total_current_liabilities: e["totalCurrentLiabilities"].as_f64().unwrap_or(0.0),
        long_term_debt: e["longTermDebt"].as_f64().unwrap_or(0.0),
        total_non_current_liabilities: e["totalNonCurrentLiabilities"].as_f64().unwrap_or(0.0),
        total_liabilities: e["totalLiabilities"].as_f64().unwrap_or(0.0),
        common_stock: e["commonStock"].as_f64().unwrap_or(0.0),
        retained_earnings: e["retainedEarnings"].as_f64().unwrap_or(0.0),
        total_equity: e["totalStockholdersEquity"].as_f64().unwrap_or(0.0),
        total_debt: e["totalDebt"].as_f64().unwrap_or(0.0),
        net_debt: e["netDebt"].as_f64().unwrap_or(0.0),
    }).collect();
    Ok(rows)
}

/// FMP /cash-flow-statement/{symbol} — up to 20 historical periods.
pub async fn fetch_fmp_cash_flow(
    client: &reqwest::Client,
    symbol: &str,
    period: &str,
    fmp_key: &str,
) -> Result<Vec<CashFlowStatement>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/cash-flow-statement/{}?period={}&limit=20&apikey={}",
        symbol, period, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP cash flow failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP cash flow: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP cash flow parse: {e}"))?;
    let rows = arr.into_iter().map(|e| CashFlowStatement {
        date: e["date"].as_str().unwrap_or("").to_string(),
        period: e["period"].as_str().unwrap_or("").to_string(),
        net_income: e["netIncome"].as_f64().unwrap_or(0.0),
        depreciation_amortization: e["depreciationAndAmortization"].as_f64().unwrap_or(0.0),
        stock_based_comp: e["stockBasedCompensation"].as_f64().unwrap_or(0.0),
        change_working_capital: e["changeInWorkingCapital"].as_f64().unwrap_or(0.0),
        cash_from_operations: e["operatingCashFlow"].as_f64().unwrap_or(0.0),
        capex: e["capitalExpenditure"].as_f64().unwrap_or(0.0),
        acquisitions: e["acquisitionsNet"].as_f64().unwrap_or(0.0),
        investments_purchases: e["purchasesOfInvestments"].as_f64().unwrap_or(0.0),
        cash_from_investing: e["netCashUsedForInvestingActivites"].as_f64().unwrap_or(0.0),
        debt_repayment: e["debtRepayment"].as_f64().unwrap_or(0.0),
        dividends_paid: e["dividendsPaid"].as_f64().unwrap_or(0.0),
        stock_repurchases: e["commonStockRepurchased"].as_f64().unwrap_or(0.0),
        cash_from_financing: e["netCashUsedProvidedByFinancingActivities"].as_f64().unwrap_or(0.0),
        net_change_cash: e["netChangeInCash"].as_f64().unwrap_or(0.0),
        free_cash_flow: e["freeCashFlow"].as_f64().unwrap_or(0.0),
    }).collect();
    Ok(rows)
}

/// Convenience: fetch the full FA bundle (all 3 statements × annual+quarterly) in one call.
/// 6 FMP calls, 400 ms between each = ~2.4 s per symbol.
pub async fn fetch_fmp_financial_bundle(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<FinancialStatements, String> {
    let mut bundle = FinancialStatements::default();
    bundle.income_annual = fetch_fmp_income_statement(client, symbol, "annual", fmp_key).await.unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.income_quarterly = fetch_fmp_income_statement(client, symbol, "quarter", fmp_key).await.unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.balance_annual = fetch_fmp_balance_sheet(client, symbol, "annual", fmp_key).await.unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.balance_quarterly = fetch_fmp_balance_sheet(client, symbol, "quarter", fmp_key).await.unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.cashflow_annual = fetch_fmp_cash_flow(client, symbol, "annual", fmp_key).await.unwrap_or_default();
    tokio::time::sleep(std::time::Duration::from_millis(400)).await;
    bundle.cashflow_quarterly = fetch_fmp_cash_flow(client, symbol, "quarter", fmp_key).await.unwrap_or_default();
    Ok(bundle)
}

/// Finnhub /stock/executive — company officers with compensation.
pub async fn fetch_finnhub_executives(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<Executive>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/executive")
        .query(&[("symbol", symbol), ("token", token)])
        .send().await
        .map_err(|e| format!("Finnhub executives failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub executives: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Finnhub executives parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["executive"].as_array() {
        for e in arr {
            rows.push(Executive {
                name: e["name"].as_str().unwrap_or("").to_string(),
                position: e["position"].as_str().unwrap_or("").to_string(),
                age: e["age"].as_i64().unwrap_or(0) as i32,
                sex: e["sex"].as_str().unwrap_or("").to_string(),
                since: e["since"].as_str().unwrap_or("").to_string(),
                compensation: e["compensation"].as_f64().unwrap_or(0.0),
                year: e["year"].as_i64().unwrap_or(0) as i32,
            });
        }
    }
    Ok(rows)
}

/// CFTC Socrata — Commitments of Traders, Legacy Futures combined.
/// Public JSON endpoint, no API key. Returns one row per market for the most recent report date.
/// WoW change in non-commercial net is computed from the prior week found in the same payload.
pub async fn fetch_cftc_cot(
    client: &reqwest::Client,
) -> Result<Vec<CotReport>, String> {
    // Legacy futures-only combined. Ordered by report date descending so the first rows
    // define the latest week, subsequent rows include the prior week for WoW delta.
    let url = "https://publicreporting.cftc.gov/resource/6dca-aqww.json?\
               $limit=2000&$order=report_date_as_yyyy_mm_dd DESC";
    let resp = client.get(url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
        .send().await
        .map_err(|e| format!("CFTC COT failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("CFTC COT: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("CFTC COT parse: {e}"))?;
    if arr.is_empty() { return Ok(vec![]); }

    // Latest report date is the max date seen in the payload (rows come sorted DESC but be safe).
    let latest_date = arr.iter()
        .filter_map(|e| e["report_date_as_yyyy_mm_dd"].as_str())
        .map(|s| s.chars().take(10).collect::<String>())
        .max()
        .unwrap_or_default();
    if latest_date.is_empty() { return Ok(vec![]); }

    // For each market, remember the first (latest) non-commercial net and the first *prior-week* net.
    use std::collections::HashMap;
    let mut prior: HashMap<String, f64> = HashMap::new();
    for e in arr.iter() {
        let market = e["market_and_exchange_names"].as_str().unwrap_or("").to_string();
        if market.is_empty() { continue; }
        let date: String = e["report_date_as_yyyy_mm_dd"].as_str().unwrap_or("").chars().take(10).collect();
        if date == latest_date { continue; }
        let nc_net = socrata_f64(&e["noncomm_positions_long_all"]) - socrata_f64(&e["noncomm_positions_short_all"]);
        prior.entry(market).or_insert(nc_net);
    }

    // Build the latest-week rows.
    let mut rows = Vec::new();
    for e in arr.iter() {
        let date: String = e["report_date_as_yyyy_mm_dd"].as_str().unwrap_or("").chars().take(10).collect();
        if date != latest_date { continue; }
        let market = e["market_and_exchange_names"].as_str().unwrap_or("").to_string();
        if market.is_empty() { continue; }
        let nc_long = socrata_f64(&e["noncomm_positions_long_all"]);
        let nc_short = socrata_f64(&e["noncomm_positions_short_all"]);
        let net = nc_long - nc_short;
        let prev = prior.get(&market).copied().unwrap_or(net);
        rows.push(CotReport {
            market_name: market,
            market_code: e["cftc_contract_market_code"].as_str().unwrap_or("").to_string(),
            report_date: date,
            open_interest: socrata_f64(&e["open_interest_all"]),
            noncomm_long: nc_long,
            noncomm_short: nc_short,
            // Socrata column name intentionally has the typo from the CFTC source feed.
            noncomm_spreads: socrata_f64(&e["noncomm_postions_spread_all"]),
            comm_long: socrata_f64(&e["comm_positions_long_all"]),
            comm_short: socrata_f64(&e["comm_positions_short_all"]),
            nonrept_long: socrata_f64(&e["nonrept_positions_long_all"]),
            nonrept_short: socrata_f64(&e["nonrept_positions_short_all"]),
            noncomm_net: net,
            noncomm_net_change: net - prev,
        });
    }
    rows.sort_by(|a, b| a.market_name.cmp(&b.market_name));
    Ok(rows)
}

// ── ADR-111 fetchers ───────────────────────────────────────────────────────

/// FMP /historical-price-full/stock_split/{symbol} — historical stock splits.
pub async fn fetch_fmp_stock_splits(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<StockSplit>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/historical-price-full/stock_split/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP splits failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP splits: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("FMP splits parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["historical"].as_array() {
        for e in arr {
            let num = e["numerator"].as_f64().unwrap_or(0.0);
            let den = e["denominator"].as_f64().unwrap_or(0.0);
            let label = e["label"].as_str().map(|s| s.to_string())
                .unwrap_or_else(|| if num > 0.0 && den > 0.0 { format!("{}:{}", num, den) } else { String::new() });
            rows.push(StockSplit {
                date: e["date"].as_str().unwrap_or("").to_string(),
                label,
                numerator: num,
                denominator: den,
            });
        }
    }
    Ok(rows)
}

/// FMP /etf-holder/{symbol} — up to 1000 constituent holdings of an ETF.
pub async fn fetch_fmp_etf_holdings(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<EtfHolding>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/etf-holder/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP etf-holder failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP etf-holder: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP etf-holder parse: {e}"))?;
    let rows = arr.into_iter().map(|e| EtfHolding {
        symbol: e["asset"].as_str().unwrap_or("").to_string(),
        name: e["name"].as_str().unwrap_or("").to_string(),
        weight_pct: e["weightPercentage"].as_f64().unwrap_or(0.0),
        shares: e["sharesNumber"].as_f64().unwrap_or(0.0),
        market_value: e["marketValue"].as_f64().unwrap_or(0.0),
        updated: e["updated"].as_str().unwrap_or("").to_string(),
    }).collect();
    Ok(rows)
}

/// Finnhub /stock/recommendation — last ~12 months of monthly buy/hold/sell bucket counts.
pub async fn fetch_finnhub_recommendations(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<AnalystRecommendation>, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/recommendation")
        .query(&[("symbol", symbol), ("token", token)])
        .send().await
        .map_err(|e| format!("Finnhub recommendations failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub recommendations: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("Finnhub recommendations parse: {e}"))?;
    let rows = arr.into_iter().map(|e| AnalystRecommendation {
        period: e["period"].as_str().unwrap_or("").to_string(),
        strong_buy: e["strongBuy"].as_i64().unwrap_or(0) as i32,
        buy: e["buy"].as_i64().unwrap_or(0) as i32,
        hold: e["hold"].as_i64().unwrap_or(0) as i32,
        sell: e["sell"].as_i64().unwrap_or(0) as i32,
        strong_sell: e["strongSell"].as_i64().unwrap_or(0) as i32,
    }).collect();
    Ok(rows)
}

/// Finnhub /stock/price-target — consensus high/low/mean target snapshot.
pub async fn fetch_finnhub_price_target(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<PriceTarget, String> {
    if token.is_empty() { return Err("Finnhub API key required".into()); }
    let resp = client
        .get("https://finnhub.io/api/v1/stock/price-target")
        .query(&[("symbol", symbol), ("token", token)])
        .send().await
        .map_err(|e| format!("Finnhub price-target failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub price-target: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Finnhub price-target parse: {e}"))?;
    Ok(PriceTarget {
        symbol: symbol.to_uppercase(),
        target_high: v["targetHigh"].as_f64().unwrap_or(0.0),
        target_low: v["targetLow"].as_f64().unwrap_or(0.0),
        target_mean: v["targetMean"].as_f64().unwrap_or(0.0),
        target_median: v["targetMedian"].as_f64().unwrap_or(0.0),
        last_updated: v["lastUpdated"].as_str().unwrap_or("").chars().take(10).collect(),
        num_analysts: v["numberOfAnalysts"].as_i64().unwrap_or(0) as i32,
    })
}

/// FMP /esg-environmental-social-governance-data?symbol={sym} — historical ESG score rows.
pub async fn fetch_fmp_esg(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<EsgScore>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v4/esg-environmental-social-governance-data?symbol={}&apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP esg failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP esg: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP esg parse: {e}"))?;
    let rows = arr.into_iter().map(|e| EsgScore {
        symbol: e["symbol"].as_str().unwrap_or(symbol).to_uppercase(),
        environmental_score: e["environmentalScore"].as_f64().unwrap_or(0.0),
        social_score: e["socialScore"].as_f64().unwrap_or(0.0),
        governance_score: e["governanceScore"].as_f64().unwrap_or(0.0),
        esg_score: e["ESGScore"].as_f64().unwrap_or(0.0),
        year: e["year"].as_i64().unwrap_or(0) as i32,
    }).collect();
    Ok(rows)
}

/// FMP index constituent endpoint (/sp500_constituent, /nasdaq_constituent, /dowjones_constituent).
/// `index_code` accepts "SP500" | "NDX" | "DJIA"; mapped to the right FMP path.
pub async fn fetch_fmp_index_members(
    client: &reqwest::Client,
    index_code: &str,
    fmp_key: &str,
) -> Result<Vec<IndexMember>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let (path, idx_label) = match index_code.to_uppercase().as_str() {
        "SP500" | "SPX" | "S&P500" => ("sp500_constituent", "SP500"),
        "NDX" | "NASDAQ" | "NDX100" => ("nasdaq_constituent", "NDX"),
        "DJIA" | "DOW" | "INDU" => ("dowjones_constituent", "DJIA"),
        other => return Err(format!("Unknown index code: {}", other)),
    };
    let url = format!(
        "https://financialmodelingprep.com/api/v3/{}?apikey={}",
        path, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP index members failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP index members: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP index members parse: {e}"))?;
    let rows = arr.into_iter().map(|e| IndexMember {
        index: idx_label.to_string(),
        symbol: e["symbol"].as_str().unwrap_or("").to_uppercase(),
        name: e["name"].as_str().unwrap_or("").to_string(),
        sector: e["sector"].as_str().unwrap_or("").to_string(),
        sub_sector: e["subSector"].as_str().unwrap_or("").to_string(),
        headquarters: e["headQuarter"].as_str().unwrap_or("").to_string(),
        date_added: e["dateFirstAdded"].as_str().unwrap_or("").to_string(),
    }).collect();
    Ok(rows)
}

// ── ADR-112 Round 5 fetchers ───────────────────────────────────────────────

/// FMP /v4/insider-trading — SEC Form 4 insider trade rows (default page=0, up to 100 rows).
pub async fn fetch_fmp_insider_trades(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<InsiderTrade>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v4/insider-trading?symbol={}&page=0&apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP insider failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP insider: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP insider parse: {e}"))?;
    let rows = arr.into_iter().map(|e| {
        let shares = e["securitiesTransacted"].as_f64().unwrap_or(0.0);
        let price = e["price"].as_f64().unwrap_or(0.0);
        InsiderTrade {
            filing_date: e["filingDate"].as_str().unwrap_or("").chars().take(10).collect(),
            transaction_date: e["transactionDate"].as_str().unwrap_or("").chars().take(10).collect(),
            reporting_name: e["reportingName"].as_str().unwrap_or("").to_string(),
            transaction_type: e["transactionType"].as_str().unwrap_or("").to_string(),
            acquisition_disposition: e["acquistionOrDisposition"].as_str().unwrap_or("").to_string(),
            shares,
            price,
            value_usd: shares * price,
            shares_owned_after: e["securitiesOwned"].as_f64().unwrap_or(0.0),
            link: e["link"].as_str().unwrap_or("").to_string(),
        }
    }).collect();
    Ok(rows)
}

/// FMP /v3/institutional-holder/{symbol} — 13F-derived top holders of a stock.
pub async fn fetch_fmp_institutional_holders(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<InstitutionalHolder>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/institutional-holder/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP holders failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP holders: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP holders parse: {e}"))?;
    let rows = arr.into_iter().map(|e| InstitutionalHolder {
        holder: e["holder"].as_str().unwrap_or("").to_string(),
        shares: e["shares"].as_f64().unwrap_or(0.0),
        date_reported: e["dateReported"].as_str().unwrap_or("").chars().take(10).collect(),
        change: e["change"].as_f64().unwrap_or(0.0),
    }).collect();
    Ok(rows)
}

/// FMP /v4/shares_float?symbol=… — latest free-float / outstanding-shares snapshot.
pub async fn fetch_fmp_shares_float(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<SharesFloat, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v4/shares_float?symbol={}&apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP shares_float failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP shares_float: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("FMP shares_float parse: {e}"))?;
    // Response is a 1-element array or a bare object — handle both.
    let e = if let Some(first) = v.as_array().and_then(|a| a.first()) { first.clone() } else { v };
    Ok(SharesFloat {
        symbol: e["symbol"].as_str().unwrap_or(symbol).to_uppercase(),
        date: e["date"].as_str().unwrap_or("").chars().take(10).collect(),
        free_float_pct: e["freeFloat"].as_f64().unwrap_or(0.0),
        float_shares: e["floatShares"].as_f64().unwrap_or(0.0),
        outstanding_shares: e["outstandingShares"].as_f64().unwrap_or(0.0),
        source: e["source"].as_str().unwrap_or("").to_string(),
    })
}

/// FMP /v3/historical-price-full/{symbol} — up to ~5 years of daily OHLCV.
/// `limit` is applied client-side after parsing (FMP returns all history by default).
pub async fn fetch_fmp_historical_price(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
    limit: usize,
) -> Result<Vec<HistoricalPriceRow>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/historical-price-full/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP historical failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP historical: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("FMP historical parse: {e}"))?;
    let mut rows = Vec::new();
    if let Some(arr) = v["historical"].as_array() {
        for e in arr.iter().take(limit.max(1)) {
            rows.push(HistoricalPriceRow {
                date: e["date"].as_str().unwrap_or("").to_string(),
                open: e["open"].as_f64().unwrap_or(0.0),
                high: e["high"].as_f64().unwrap_or(0.0),
                low: e["low"].as_f64().unwrap_or(0.0),
                close: e["close"].as_f64().unwrap_or(0.0),
                adj_close: e["adjClose"].as_f64().unwrap_or(0.0),
                volume: e["volume"].as_f64().unwrap_or(0.0),
                change: e["change"].as_f64().unwrap_or(0.0),
                change_pct: e["changePercent"].as_f64().unwrap_or(0.0),
            });
        }
    }
    Ok(rows)
}

/// FMP /v3/earning_surprise/{symbol} — quarterly actual-vs-estimate EPS history.
pub async fn fetch_fmp_earnings_surprises(
    client: &reqwest::Client,
    symbol: &str,
    fmp_key: &str,
) -> Result<Vec<EarningsSurprise>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/earning_surprise/{}?apikey={}",
        symbol, fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP surprise failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP surprise: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP surprise parse: {e}"))?;
    let rows = arr.into_iter().map(|e| {
        let actual = e["actualEarningResult"].as_f64().unwrap_or(0.0);
        let est = e["estimatedEarning"].as_f64().unwrap_or(0.0);
        let surprise = actual - est;
        let surprise_pct = if est.abs() > 1e-9 { (surprise / est.abs()) * 100.0 } else { 0.0 };
        EarningsSurprise {
            date: e["date"].as_str().unwrap_or("").to_string(),
            symbol: e["symbol"].as_str().unwrap_or(symbol).to_uppercase(),
            eps_actual: actual,
            eps_estimate: est,
            surprise,
            surprise_pct,
        }
    }).collect();
    Ok(rows)
}

// ── ADR-113 Round 6 fetchers ───────────────────────────────────────────────

/// Yahoo batch-quote the WORLD_INDICES_UNIVERSE tickers for the WEI dashboard.
/// Returns rows in the universe's declared order so the UI grouping stays stable.
pub async fn fetch_world_indices(
    client: &reqwest::Client,
) -> Result<Vec<WorldIndex>, String> {
    let tickers: Vec<&str> = WORLD_INDICES_UNIVERSE.iter().map(|(t, _, _)| *t).collect();
    let quotes = fetch_yahoo_quotes(client, &tickers).await?;
    let mut by_sym: std::collections::HashMap<String, (f64, f64, f64)> =
        std::collections::HashMap::new();
    for (sym, price, change, pct) in quotes {
        by_sym.insert(sym, (price, change, pct));
    }
    let rows: Vec<WorldIndex> = WORLD_INDICES_UNIVERSE.iter().map(|(t, d, r)| {
        let (price, change, pct) = by_sym.get(*t).cloned().unwrap_or((0.0, 0.0, 0.0));
        WorldIndex {
            ticker: (*t).to_string(),
            display: (*d).to_string(),
            region: (*r).to_string(),
            price,
            change,
            change_pct: pct,
        }
    }).collect();
    Ok(rows)
}

/// Helper — parse a single FMP mover row into MarketMover.
fn parse_fmp_mover(e: &serde_json::Value) -> MarketMover {
    let price = e["price"].as_f64().unwrap_or(0.0);
    let change = e["change"].as_f64()
        .or_else(|| e["changes"].as_f64())
        .unwrap_or(0.0);
    // FMP often returns "changesPercentage" as a string like "-5.60%"
    let change_pct = e["changesPercentage"].as_f64()
        .or_else(|| e["changesPercentage"].as_str().map(|s| {
            s.trim_matches(|c: char| c == '%' || c.is_whitespace()).parse::<f64>().unwrap_or(0.0)
        }))
        .unwrap_or(0.0);
    MarketMover {
        symbol: e["symbol"].as_str().unwrap_or("").to_string(),
        name: e["name"].as_str().unwrap_or("").to_string(),
        price,
        change,
        change_pct,
        volume: e["volume"].as_f64().unwrap_or(0.0),
    }
}

/// FMP /v3/stock_market/{gainers|losers|actives} — bundled into one MarketMovers.
pub async fn fetch_fmp_market_movers(
    client: &reqwest::Client,
    fmp_key: &str,
) -> Result<MarketMovers, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let mut out = MarketMovers::default();
    for (bucket, field) in [("gainers", 0), ("losers", 1), ("actives", 2)] {
        let url = format!(
            "https://financialmodelingprep.com/api/v3/stock_market/{}?apikey={}",
            bucket, fmp_key
        );
        let resp = client.get(&url).send().await
            .map_err(|e| format!("FMP {} failed: {}", bucket, e))?;
        if !resp.status().is_success() {
            return Err(format!("FMP {}: HTTP {}", bucket, resp.status()));
        }
        let arr: Vec<serde_json::Value> = resp.json().await
            .map_err(|e| format!("FMP {} parse: {}", bucket, e))?;
        let rows: Vec<MarketMover> = arr.iter().map(parse_fmp_mover).collect();
        match field {
            0 => out.gainers = rows,
            1 => out.losers = rows,
            _ => out.actives = rows,
        }
    }
    Ok(out)
}

/// FMP /v3/sector-performance — intraday performance for all GICS sectors.
pub async fn fetch_fmp_sector_performance(
    client: &reqwest::Client,
    fmp_key: &str,
) -> Result<Vec<SectorPerformance>, String> {
    if fmp_key.is_empty() { return Err("FMP API key required".into()); }
    let url = format!(
        "https://financialmodelingprep.com/api/v3/sector-performance?apikey={}",
        fmp_key
    );
    let resp = client.get(&url).send().await
        .map_err(|e| format!("FMP sector-performance failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP sector-performance: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp.json().await
        .map_err(|e| format!("FMP sector-performance parse: {e}"))?;
    let rows: Vec<SectorPerformance> = arr.into_iter().map(|e| {
        let sector = e["sector"].as_str().unwrap_or("").to_string();
        // FMP returns "changesPercentage" as a "1.23%" string.
        let pct_raw = e["changesPercentage"].as_str().unwrap_or("0");
        let change_pct = pct_raw
            .trim_matches(|c: char| c == '%' || c.is_whitespace())
            .parse::<f64>()
            .unwrap_or(0.0);
        SectorPerformance { sector, change_pct }
    }).collect();
    Ok(rows)
}

/// Build a WACC snapshot by combining FMP profile (beta + market cap) with the
/// latest cached FA income/balance data (interest expense, total debt, tax rate)
/// and a caller-supplied risk-free rate (typically 10Y Treasury yield %).
///
/// This is a pure derivation: it does NOT hit the network.  Callers should
/// fetch the inputs first (profile, financials, yield curve) then pass them in.
pub fn compute_wacc_snapshot(
    symbol: &str,
    as_of: &str,
    beta: f64,
    market_cap: f64,
    risk_free_pct: f64,
    total_debt: f64,
    interest_expense: f64,
    effective_tax_rate_pct: f64,
) -> WaccSnapshot {
    let erp = DEFAULT_EQUITY_RISK_PREMIUM_PCT;
    let cost_of_equity_pct = risk_free_pct + beta * erp;

    let pre_tax_cost_of_debt_pct = if total_debt.abs() > 1e-6 {
        (interest_expense.abs() / total_debt) * 100.0
    } else { 0.0 };

    let tax_rate_pct = effective_tax_rate_pct.clamp(0.0, 60.0);
    let after_tax_cost_of_debt_pct = pre_tax_cost_of_debt_pct * (1.0 - tax_rate_pct / 100.0);

    let total_cap = market_cap + total_debt;
    let equity_weight = if total_cap > 1e-6 { market_cap / total_cap } else { 1.0 };
    let debt_weight = 1.0 - equity_weight;
    let wacc_pct = equity_weight * cost_of_equity_pct + debt_weight * after_tax_cost_of_debt_pct;

    WaccSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        beta,
        risk_free_pct,
        equity_risk_premium_pct: erp,
        cost_of_equity_pct,
        pre_tax_cost_of_debt_pct,
        tax_rate_pct,
        after_tax_cost_of_debt_pct,
        market_cap,
        total_debt,
        equity_weight,
        debt_weight,
        wacc_pct,
    }
}

// ── ADR-114 Round 7 — WCR fetcher ─────────────────────────────────────────

/// Fetch the hardcoded FX-majors universe through Yahoo and return the rows
/// in the order declared by `FX_MAJORS_UNIVERSE`.
pub async fn fetch_currency_rates(
    client: &reqwest::Client,
) -> Result<Vec<CurrencyRate>, String> {
    let tickers: Vec<&str> = FX_MAJORS_UNIVERSE.iter().map(|(t, _, _, _, _)| *t).collect();
    let quotes = fetch_yahoo_quotes(client, &tickers).await?;

    use std::collections::HashMap;
    let by_ticker: HashMap<String, (f64, f64, f64)> = quotes.into_iter()
        .map(|(t, p, c, pct)| (t, (p, c, pct)))
        .collect();

    let mut out = Vec::with_capacity(FX_MAJORS_UNIVERSE.len());
    for (tk, display, base, quote, region) in FX_MAJORS_UNIVERSE.iter() {
        let (price, change, change_pct) = by_ticker.get(*tk)
            .copied()
            .unwrap_or((0.0, 0.0, 0.0));
        out.push(CurrencyRate {
            ticker: (*tk).to_string(),
            display: (*display).to_string(),
            base: (*base).to_string(),
            quote: (*quote).to_string(),
            region: (*region).to_string(),
            price,
            change,
            change_pct,
        });
    }
    Ok(out)
}

// ── ADR-114 Round 7 — BETA compute ────────────────────────────────────────

/// Compute an OLS regression of symbol log-returns on market log-returns.
/// Returns (beta, alpha_per_period, r_squared, correlation, n).
/// Pure function, no I/O. Daily returns expected; alpha is per-period
/// (caller annualizes as needed).
fn ols_regression(symbol_returns: &[f64], market_returns: &[f64]) -> (f64, f64, f64, f64, usize) {
    let n = symbol_returns.len().min(market_returns.len());
    if n < 10 { return (0.0, 0.0, 0.0, 0.0, n); }
    let mean_s: f64 = symbol_returns.iter().take(n).sum::<f64>() / n as f64;
    let mean_m: f64 = market_returns.iter().take(n).sum::<f64>() / n as f64;

    let mut cov = 0.0_f64;
    let mut var_m = 0.0_f64;
    let mut var_s = 0.0_f64;
    for i in 0..n {
        let ds = symbol_returns[i] - mean_s;
        let dm = market_returns[i] - mean_m;
        cov += ds * dm;
        var_m += dm * dm;
        var_s += ds * ds;
    }
    if var_m <= 1e-12 { return (0.0, 0.0, 0.0, 0.0, n); }
    let beta = cov / var_m;
    let alpha = mean_s - beta * mean_m;

    // R² (symbol variance explained by market) = β² · var_m / var_s
    let r_squared = if var_s > 1e-12 { (beta * beta) * var_m / var_s } else { 0.0 };
    let correlation = if var_m > 1e-12 && var_s > 1e-12 {
        cov / (var_m.sqrt() * var_s.sqrt())
    } else { 0.0 };

    (beta, alpha, r_squared.clamp(0.0, 1.0), correlation, n)
}

/// Compute log-returns from a sequence of closes (newest-first or oldest-first
/// both work — the function only cares about adjacent differences). Result is
/// in the same order as the input (length = len - 1).
fn log_returns(closes: &[f64]) -> Vec<f64> {
    if closes.len() < 2 { return Vec::new(); }
    closes.windows(2)
        .map(|w| if w[0] > 0.0 && w[1] > 0.0 { (w[1] / w[0]).ln() } else { 0.0 })
        .collect()
}

/// Compute a per-symbol beta snapshot against a market benchmark using
/// cached FMP historical price rows for both series. Caller fetches the bars
/// once (or reuses the HP cache) and hands them in. The bars must be sorted
/// **newest-first** (FMP returns them that way by default).
///
/// We compute 1Y / 3Y / 5Y windows using the trailing N trading days.
/// Windows that don't have enough overlapping data are skipped silently.
pub fn compute_beta_snapshot(
    symbol: &str,
    market_ticker: &str,
    as_of: &str,
    sym_bars_newest_first: &[HistoricalPriceRow],
    mkt_bars_newest_first: &[HistoricalPriceRow],
) -> BetaSnapshot {
    use std::collections::HashMap;
    // Intersect by date to make returns directly comparable.
    let mkt_by_date: HashMap<&str, f64> = mkt_bars_newest_first.iter()
        .map(|b| (b.date.as_str(), b.close))
        .collect();
    let mut paired: Vec<(String, f64, f64)> = sym_bars_newest_first.iter()
        .filter_map(|b| mkt_by_date.get(b.date.as_str())
            .map(|m| (b.date.clone(), b.close, *m)))
        .collect();
    // Sort ascending by date so the log_returns helper produces chronological returns.
    paired.sort_by(|a, b| a.0.cmp(&b.0));

    let sym_closes: Vec<f64> = paired.iter().map(|(_, s, _)| *s).collect();
    let mkt_closes: Vec<f64> = paired.iter().map(|(_, _, m)| *m).collect();
    let sym_rets = log_returns(&sym_closes);
    let mkt_rets = log_returns(&mkt_closes);

    let mut windows = Vec::new();
    let mut note = String::new();

    for (label, days) in [("1Y", 252usize), ("3Y", 756), ("5Y", 1260)] {
        let n_available = sym_rets.len().min(mkt_rets.len());
        if n_available == 0 {
            continue;
        }
        // Use the most recent `days` returns (tail slice) — sym_rets/mkt_rets
        // are ordered chronologically (oldest first, newest last).
        let take = days.min(n_available);
        let s_slice = &sym_rets[n_available - take..];
        let m_slice = &mkt_rets[n_available - take..];
        let (beta, alpha, r2, corr, n_obs) = ols_regression(s_slice, m_slice);
        if n_obs < 20 {
            if note.is_empty() && label == "1Y" {
                note = format!("insufficient overlapping data (n={n_obs}) for stable beta");
            }
            continue;
        }
        windows.push(BetaWindow {
            window_label: label.to_string(),
            window_days: days,
            beta,
            alpha_pct: alpha * 252.0 * 100.0, // annualize daily alpha
            r_squared: r2,
            n_observations: n_obs,
            correlation: corr,
        });
    }

    BetaSnapshot {
        symbol: symbol.to_uppercase(),
        market_ticker: market_ticker.to_string(),
        as_of: as_of.to_string(),
        windows,
        note,
    }
}

// ── ADR-114 Round 7 — DDM compute ─────────────────────────────────────────

/// Compute a Gordon Growth dividend-discount-model snapshot from cached
/// dividend history and a required return (typically WACC or cost of equity).
///
/// Dividends are newest-first (matching `get_dividends`). Growth rate is
/// inferred from the 5-year dividend CAGR when at least 5 annual dividends
/// are available, with fallback to a clamped 3% assumption. If r ≤ g, the
/// Gordon formula degenerates — we return implied_price = 0.0 with a note.
pub fn compute_ddm_snapshot(
    symbol: &str,
    as_of: &str,
    dividends_newest_first: &[DividendRecord],
    required_return_pct: f64,
    return_source: &str,
) -> DdmSnapshot {
    if dividends_newest_first.is_empty() {
        return DdmSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            method: "Gordon Growth".to_string(),
            note: "no dividend history on file".to_string(),
            ..Default::default()
        };
    }

    // Trailing 4-quarter dividend ($ per share). We use adjusted_amount
    // so split adjustments don't distort the growth rate.
    let div_amount = |d: &DividendRecord| -> f64 {
        if d.adjusted_amount > 0.0 { d.adjusted_amount } else { d.amount }
    };
    let annual_dividend: f64 = dividends_newest_first.iter()
        .take(4)
        .map(div_amount)
        .sum();

    // Infer growth: bucket dividends by ex-date year, then CAGR over 5 years
    // if possible. Each bucket sums the quarterly payments for that year.
    use std::collections::BTreeMap;
    let mut by_year: BTreeMap<i32, f64> = BTreeMap::new();
    for d in dividends_newest_first.iter() {
        // ex_date like "2025-10-31" — parse the 4-digit prefix.
        if let Some(year_str) = d.ex_date.get(..4) {
            if let Ok(y) = year_str.parse::<i32>() {
                *by_year.entry(y).or_insert(0.0) += div_amount(d);
            }
        }
    }
    let years_sorted: Vec<(i32, f64)> = by_year.into_iter().collect();
    let (implied_growth_pct, growth_source) = if years_sorted.len() >= 6 {
        // Use 5-year CAGR: years_sorted.last() vs years_sorted[len-6]
        let end = years_sorted[years_sorted.len() - 2].1; // second-to-last (last might be partial)
        let start_idx = years_sorted.len().saturating_sub(7);
        let start = years_sorted[start_idx].1;
        if start > 1e-9 && end > 1e-9 {
            let n_years = (years_sorted.len() - 2 - start_idx) as f64;
            let cagr = (end / start).powf(1.0 / n_years.max(1.0)) - 1.0;
            (cagr.clamp(-0.20, 0.20) * 100.0, format!("{:.0}Y dividend CAGR", n_years))
        } else {
            (3.0, "fallback (insufficient history)".to_string())
        }
    } else if years_sorted.len() >= 3 {
        // Short history: compare oldest full year to newest full year.
        let end = years_sorted[years_sorted.len() - 2].1;
        let start = years_sorted[0].1;
        if start > 1e-9 && end > 1e-9 {
            let n_years = (years_sorted.len() - 2) as f64;
            let cagr = (end / start).powf(1.0 / n_years.max(1.0)) - 1.0;
            (cagr.clamp(-0.20, 0.20) * 100.0, format!("{:.0}Y dividend CAGR", n_years))
        } else {
            (3.0, "fallback (short history)".to_string())
        }
    } else {
        (3.0, "fallback (no growth history)".to_string())
    };

    // Gordon Growth: P = D1 / (r - g), where D1 = D0 * (1 + g).
    let g = implied_growth_pct / 100.0;
    let r = required_return_pct / 100.0;
    let (implied_price, note) = if r > g + 0.005 && annual_dividend > 0.0 {
        let d1 = annual_dividend * (1.0 + g);
        (d1 / (r - g), String::new())
    } else if annual_dividend <= 0.0 {
        (0.0, "annual dividend is zero — Gordon Growth not applicable".to_string())
    } else {
        (0.0, format!(
            "required return {:.2}% ≤ growth {:.2}% — Gordon formula diverges",
            required_return_pct, implied_growth_pct))
    };

    DdmSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        annual_dividend,
        implied_growth_pct,
        required_return_pct,
        growth_source,
        return_source: return_source.to_string(),
        implied_price,
        method: "Gordon Growth".to_string(),
        note,
    }
}

// ── ADR-114 Round 7 — RV compute (relative valuation peer matrix) ─────────

/// One input row for the relative-valuation calculator: a metric name plus
/// the subject's value and a list of peer values. Caller builds this from
/// cached fundamentals; the function is pure.
pub struct RvMetricInput<'a> {
    pub metric: &'a str,
    pub value: Option<f64>,
    pub peer_values: Vec<f64>,
}

/// Compute a `RelativeValuation` snapshot from a list of metric inputs.
/// Skips metrics where the subject has no value or the peer set has fewer
/// than 3 observations (same threshold the packet's sector-peer block uses).
pub fn compute_relative_valuation(
    symbol: &str,
    sector: &str,
    as_of: &str,
    metrics: &[RvMetricInput<'_>],
) -> RelativeValuation {
    let mut rows = Vec::new();
    let mut max_peer_count = 0;

    for m in metrics {
        let val = match m.value { Some(v) if v.is_finite() => v, _ => continue };
        let mut peers: Vec<f64> = m.peer_values.iter().copied()
            .filter(|x| x.is_finite())
            .collect();
        if peers.len() < 3 { continue; }
        peers.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = peers.len();
        max_peer_count = max_peer_count.max(n);

        let median = peers[n / 2];
        let low = peers[0];
        let high = peers[n - 1];
        let mean = peers.iter().sum::<f64>() / n as f64;
        let variance = peers.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / n as f64;
        let stdev = variance.sqrt();
        let z_score = if stdev > 1e-9 { (val - mean) / stdev } else { 0.0 };
        let below = peers.iter().filter(|p| **p < val).count();
        let percentile = (below as f64 / n as f64) * 100.0;

        rows.push(RvMetricRow {
            metric: m.metric.to_string(),
            value: val,
            peer_median: median,
            peer_low: low,
            peer_high: high,
            z_score,
            percentile,
        });
    }

    RelativeValuation {
        symbol: symbol.to_uppercase(),
        sector: sector.to_string(),
        as_of: as_of.to_string(),
        peer_count: max_peer_count,
        rows,
    }
}

// ── ADR-114 Round 7 — FIGI (OpenFIGI) fetcher ─────────────────────────────

/// Fetch OpenFIGI identifiers for a symbol. OpenFIGI is a free service run by
/// Bloomberg — no API key required for reasonable volumes. We POST the
/// ticker as an exchange-code lookup against US common-stock space.
pub async fn fetch_openfigi_identifiers(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<Vec<FigiIdentifier>, String> {
    let body = serde_json::json!([{
        "idType": "TICKER",
        "idValue": symbol.to_uppercase(),
        "marketSecDes": "Equity"
    }]);
    let resp = client.post("https://api.openfigi.com/v3/mapping")
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await
        .map_err(|e| format!("OpenFIGI request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("OpenFIGI: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("OpenFIGI parse: {e}"))?;
    let outer = v.as_array().ok_or_else(|| "OpenFIGI: expected array".to_string())?;
    let mut out = Vec::new();
    for entry in outer {
        if let Some(data) = entry.get("data").and_then(|d| d.as_array()) {
            for row in data {
                out.push(FigiIdentifier {
                    figi: row["figi"].as_str().unwrap_or("").to_string(),
                    name: row["name"].as_str().unwrap_or("").to_string(),
                    ticker: row["ticker"].as_str().unwrap_or("").to_string(),
                    exch_code: row["exchCode"].as_str().unwrap_or("").to_string(),
                    composite_figi: row["compositeFIGI"].as_str().unwrap_or("").to_string(),
                    share_class_figi: row["shareClassFIGI"].as_str().unwrap_or("").to_string(),
                    security_type: row["securityType"].as_str().unwrap_or("").to_string(),
                    security_type_2: row["securityType2"].as_str().unwrap_or("").to_string(),
                    market_sector: row["marketSector"].as_str().unwrap_or("").to_string(),
                    security_description: row["securityDescription"].as_str().unwrap_or("").to_string(),
                });
            }
        }
    }
    Ok(out)
}

// ── ADR-115 Round 8 — HRA compute (historical return + risk) ──────────────

/// Compute an `HraSnapshot` from a chronologically-ordered slice of bars
/// (oldest → newest). Returns periods are simple-return (close₀→closeₙ),
/// annualized into CAGR for windows ≥ 252 trading days. Max drawdown is
/// computed over the full available history; Sharpe/Sortino use daily
/// log-returns annualized with the supplied risk-free rate.
pub fn compute_hra_snapshot(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
    risk_free_pct: f64,
) -> HraSnapshot {
    if bars_oldest_first.len() < 2 {
        return HraSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            note: "insufficient bar history (need ≥ 2 bars)".to_string(),
            ..Default::default()
        };
    }
    let n = bars_oldest_first.len();
    let last_close = bars_oldest_first[n - 1].close;
    let first_close = bars_oldest_first[0].close;

    // Helper: return (pct) from a trading-day lookback (uses adjusted close
    // when available so splits/dividends don't poison the return).
    let px = |i: usize| -> f64 {
        let b = &bars_oldest_first[i];
        if b.adj_close > 0.0 { b.adj_close } else { b.close }
    };
    let last_px = px(n - 1);

    let mut windows: Vec<HraWindow> = Vec::new();
    let add_trading_window = |windows: &mut Vec<HraWindow>, label: &str, days: usize| {
        if n <= days { return; }
        let start = n - 1 - days;
        let start_px = px(start);
        if start_px <= 0.0 { return; }
        let ret = (last_px / start_px - 1.0) * 100.0;
        let cagr = if days >= 252 {
            let years = days as f64 / 252.0;
            ((last_px / start_px).powf(1.0 / years) - 1.0) * 100.0
        } else { ret };
        windows.push(HraWindow {
            label: label.to_string(),
            trading_days: days,
            return_pct: ret,
            cagr_pct: cagr,
            n_observations: days,
        });
    };
    add_trading_window(&mut windows, "1D",   1);
    add_trading_window(&mut windows, "5D",   5);
    add_trading_window(&mut windows, "1M",   21);
    add_trading_window(&mut windows, "3M",   63);
    add_trading_window(&mut windows, "6M",   126);
    add_trading_window(&mut windows, "1Y",   252);
    add_trading_window(&mut windows, "3Y",   756);
    add_trading_window(&mut windows, "5Y",   1260);

    // YTD: first bar whose date starts with current year.
    let year_prefix = as_of.get(..4).unwrap_or("");
    if !year_prefix.is_empty() {
        if let Some(ytd_start) = bars_oldest_first.iter()
            .position(|b| b.date.starts_with(year_prefix))
        {
            let start_px = px(ytd_start);
            if start_px > 0.0 {
                let ret = (last_px / start_px - 1.0) * 100.0;
                windows.push(HraWindow {
                    label: "YTD".to_string(),
                    trading_days: 0,
                    return_pct: ret,
                    cagr_pct: ret,
                    n_observations: n - ytd_start,
                });
            }
        }
    }

    // ITD: full span.
    if first_close > 0.0 {
        let ret = (last_px / first_close - 1.0) * 100.0;
        let years = (n as f64 / 252.0).max(1.0 / 252.0);
        let cagr = ((last_px / first_close).powf(1.0 / years) - 1.0) * 100.0;
        windows.push(HraWindow {
            label: "ITD".to_string(),
            trading_days: n - 1,
            return_pct: ret,
            cagr_pct: cagr,
            n_observations: n,
        });
    }

    // Max drawdown: walk forward tracking running peak.
    let mut peak = px(0);
    let mut peak_idx = 0usize;
    let mut max_dd = 0.0f64;
    let mut dd_peak_idx = 0usize;
    let mut dd_trough_idx = 0usize;
    for i in 1..n {
        let p = px(i);
        if p > peak { peak = p; peak_idx = i; }
        if peak > 0.0 {
            let dd = (p / peak - 1.0) * 100.0;
            if dd < max_dd {
                max_dd = dd;
                dd_peak_idx = peak_idx;
                dd_trough_idx = i;
            }
        }
    }

    // Daily log returns → annualized volatility and Sharpe/Sortino.
    let mut log_rets: Vec<f64> = Vec::with_capacity(n.saturating_sub(1));
    for i in 1..n {
        let p0 = px(i - 1);
        let p1 = px(i);
        if p0 > 0.0 && p1 > 0.0 { log_rets.push((p1 / p0).ln()); }
    }
    let (vol_ann_pct, sharpe, sortino) = if log_rets.len() >= 20 {
        let m = log_rets.iter().sum::<f64>() / log_rets.len() as f64;
        let var = log_rets.iter().map(|r| (r - m).powi(2)).sum::<f64>() / log_rets.len() as f64;
        let sd = var.sqrt();
        let down: Vec<f64> = log_rets.iter().copied().filter(|r| *r < 0.0).collect();
        let dsd = if down.is_empty() { sd } else {
            let dm = down.iter().sum::<f64>() / down.len() as f64;
            (down.iter().map(|r| (r - dm).powi(2)).sum::<f64>() / down.len() as f64).sqrt()
        };
        let rf_daily = (risk_free_pct / 100.0) / 252.0;
        let sharpe = if sd > 1e-9 { (m - rf_daily) / sd * (252.0f64).sqrt() } else { 0.0 };
        let sortino = if dsd > 1e-9 { (m - rf_daily) / dsd * (252.0f64).sqrt() } else { 0.0 };
        (sd * (252.0f64).sqrt() * 100.0, sharpe, sortino)
    } else {
        (0.0, 0.0, 0.0)
    };

    let itd_cagr = windows.iter().find(|w| w.label == "ITD").map(|w| w.cagr_pct).unwrap_or(0.0);
    let calmar = if max_dd.abs() > 1e-9 { itd_cagr / max_dd.abs() } else { 0.0 };

    HraSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        last_close,
        windows,
        max_drawdown_pct: max_dd,
        drawdown_peak_date: bars_oldest_first.get(dd_peak_idx).map(|b| b.date.clone()).unwrap_or_default(),
        drawdown_trough_date: bars_oldest_first.get(dd_trough_idx).map(|b| b.date.clone()).unwrap_or_default(),
        volatility_annual_pct: vol_ann_pct,
        sharpe_ratio: sharpe,
        sortino_ratio: sortino,
        calmar_ratio: calmar,
        risk_free_pct,
        note: String::new(),
    }
}

// ── ADR-115 Round 8 — DCF compute (Discounted Cash Flow, FCFF basis) ─────

/// Compute a multi-year DCF fair-value snapshot on a free cash flow to firm
/// (FCFF) basis. All inputs are already-cached values — this is pure compute.
///
/// Formula: EV = Σ(FCFFₜ / (1 + wacc)ᵗ) + TV / (1 + wacc)ⁿ
/// where TV = FCFFₙ × (1 + terminal_g) / (wacc − terminal_g).
/// Equity value = EV − debt + cash. Implied price = equity / shares.
#[allow(clippy::too_many_arguments)]
pub fn compute_dcf_snapshot(
    symbol: &str,
    as_of: &str,
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
) -> DcfSnapshot {
    let wacc = wacc_pct / 100.0;
    let g    = growth_pct / 100.0;
    let tg   = terminal_growth_pct / 100.0;

    if wacc <= 0.0 || shares_outstanding <= 0.0 || base_fcff.abs() < 1e-6 {
        return DcfSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            method: "DCF on FCFF".to_string(),
            base_revenue,
            base_fcff,
            growth_pct,
            terminal_growth_pct,
            wacc_pct,
            tax_rate_pct,
            projection_years,
            shares_outstanding,
            total_debt,
            cash_and_equivalents,
            note: "insufficient inputs (wacc, shares, or base fcff ≈ 0)".to_string(),
            ..Default::default()
        };
    }
    if tg + 0.005 >= wacc {
        return DcfSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            method: "DCF on FCFF".to_string(),
            base_revenue,
            base_fcff,
            growth_pct,
            terminal_growth_pct,
            wacc_pct,
            tax_rate_pct,
            projection_years,
            shares_outstanding,
            total_debt,
            cash_and_equivalents,
            note: format!("terminal growth {:.2}% ≥ WACC {:.2}% — DCF degenerate", terminal_growth_pct, wacc_pct),
            ..Default::default()
        };
    }

    let fcff_margin_pct = if base_revenue > 0.0 { base_fcff / base_revenue * 100.0 } else { 0.0 };

    let mut years: Vec<DcfYear> = Vec::with_capacity(projection_years);
    let mut pv_sum = 0.0f64;
    let mut last_fcff = base_fcff;
    let mut last_revenue = base_revenue;
    for t in 1..=projection_years {
        last_revenue *= 1.0 + g;
        last_fcff *= 1.0 + g;
        let discount = (1.0 + wacc).powi(t as i32);
        let df = 1.0 / discount;
        let pv = last_fcff * df;
        pv_sum += pv;
        years.push(DcfYear {
            year: t as i32,
            revenue: last_revenue,
            ebit: 0.0,
            nopat: 0.0,
            fcff: last_fcff,
            discount_factor: df,
            pv_fcff: pv,
        });
    }

    let terminal_value = last_fcff * (1.0 + tg) / (wacc - tg);
    let pv_terminal = terminal_value / (1.0 + wacc).powi(projection_years as i32);
    let enterprise_value = pv_sum + pv_terminal;
    let equity_value = enterprise_value - total_debt + cash_and_equivalents;
    let implied_price = if shares_outstanding > 0.0 { equity_value / shares_outstanding } else { 0.0 };

    DcfSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        method: "DCF on FCFF".to_string(),
        base_revenue,
        base_fcff,
        growth_pct,
        terminal_growth_pct,
        wacc_pct,
        tax_rate_pct,
        fcff_margin_pct,
        projection_years,
        years,
        pv_sum,
        terminal_value,
        pv_terminal,
        enterprise_value,
        total_debt,
        cash_and_equivalents,
        equity_value,
        shares_outstanding,
        implied_price,
        note: String::new(),
    }
}

// ── ADR-115 Round 8 — SVM compute (Stock Valuation Model triangulation) ──

/// Build a multi-model fair-value triangulation from the caller's cached
/// WACC / DDM / DCF / RV snapshots plus any peer-median multiples the
/// caller has already computed. All inputs are optional — rows with no
/// implied price are skipped.
pub fn compute_svm_snapshot(
    symbol: &str,
    as_of: &str,
    current_price: f64,
    ddm: Option<&DdmSnapshot>,
    dcf: Option<&DcfSnapshot>,
    peer_pe_median: Option<(f64, f64)>,           // (peer_pe, subject eps)
    peer_ev_ebitda_median: Option<(f64, f64, f64, f64, f64)>, // (peer_ev/ebitda, ebitda, debt, cash, shares)
    peer_pbook_median: Option<(f64, f64)>,        // (peer_pb, book value per share)
) -> SvmSnapshot {
    let mut rows: Vec<SvmModelRow> = Vec::new();
    let push = |rows: &mut Vec<SvmModelRow>, model: &str, implied: f64, source: String, confidence: &str| {
        if implied <= 0.0 { return; }
        let upside = if current_price > 0.0 { (implied / current_price - 1.0) * 100.0 } else { 0.0 };
        rows.push(SvmModelRow {
            model: model.to_string(),
            implied_price: implied,
            current_price,
            upside_pct: upside,
            confidence: confidence.to_string(),
            source,
        });
    };

    if let Some(d) = ddm {
        if d.implied_price > 0.0 {
            push(&mut rows, "DDM Gordon Growth", d.implied_price,
                 format!("{} · g={:.2}% · r={:.2}%", d.method, d.implied_growth_pct, d.required_return_pct),
                 "medium");
        }
    }
    if let Some(d) = dcf {
        if d.implied_price > 0.0 {
            push(&mut rows, "DCF on FCFF", d.implied_price,
                 format!("{} · WACC={:.2}% · g={:.2}% · TG={:.2}%", d.method, d.wacc_pct, d.growth_pct, d.terminal_growth_pct),
                 "medium");
        }
    }
    if let Some((peer_pe, eps)) = peer_pe_median {
        if peer_pe > 0.0 && eps > 0.0 {
            push(&mut rows, "RV peer P/E median", peer_pe * eps,
                 format!("peer median P/E {:.2}× · EPS {:.2}", peer_pe, eps), "low");
        }
    }
    if let Some((peer_evebitda, ebitda, debt, cash, shares)) = peer_ev_ebitda_median {
        if peer_evebitda > 0.0 && ebitda > 0.0 && shares > 0.0 {
            let ev_implied = peer_evebitda * ebitda;
            let equity = ev_implied - debt + cash;
            let implied = equity / shares;
            push(&mut rows, "RV peer EV/EBITDA median", implied,
                 format!("peer median EV/EBITDA {:.2}× · EBITDA {:.0}", peer_evebitda, ebitda), "low");
        }
    }
    if let Some((peer_pb, bvps)) = peer_pbook_median {
        if peer_pb > 0.0 && bvps > 0.0 {
            push(&mut rows, "RV peer P/B median", peer_pb * bvps,
                 format!("peer median P/B {:.2}× · BVPS {:.2}", peer_pb, bvps), "low");
        }
    }

    let implied: Vec<f64> = rows.iter().map(|r| r.implied_price).collect();
    let (fair_low, fair_high, fair_mid) = if implied.is_empty() {
        (0.0, 0.0, 0.0)
    } else {
        let lo = implied.iter().cloned().fold(f64::INFINITY, f64::min);
        let hi = implied.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let mid = implied.iter().sum::<f64>() / implied.len() as f64;
        (lo, hi, mid)
    };
    let upside_mid = if current_price > 0.0 && fair_mid > 0.0 { (fair_mid / current_price - 1.0) * 100.0 } else { 0.0 };

    let note = if rows.is_empty() {
        "no valuation models available — run WACC/DDM/DCF/RV first".to_string()
    } else {
        String::new()
    };

    SvmSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        current_price,
        rows,
        fair_low,
        fair_high,
        fair_mid,
        upside_mid_pct: upside_mid,
        note,
    }
}

// ── ADR-115 Round 8 — OMON fetch (Yahoo options chain) ───────────────────

/// Fetch a Yahoo options chain for a symbol. Returns all expirations Yahoo
/// is willing to give us in a single call (typically 1–12 weeklies + LEAPS).
pub async fn fetch_yahoo_options_chain(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<OptionsChainSnapshot, String> {
    let url = format!("https://query2.finance.yahoo.com/v7/finance/options/{}", symbol.to_uppercase());
    let resp = client.get(&url)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1")
        .send().await
        .map_err(|e| format!("Yahoo options request: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Yahoo options: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp.json().await
        .map_err(|e| format!("Yahoo options parse: {e}"))?;
    let today = chrono::Utc::now().format("%Y-%m-%d").to_string();
    let result = v.pointer("/optionChain/result/0")
        .ok_or_else(|| "Yahoo options: empty result".to_string())?;
    let underlying_price = result.pointer("/quote/regularMarketPrice")
        .and_then(|x| x.as_f64()).unwrap_or(0.0);

    let expiration_dates: Vec<i64> = result.get("expirationDates")
        .and_then(|x| x.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
        .unwrap_or_default();

    // Yahoo only returns one expiration's chain per call when we don't pass
    // &date=… — we take whatever came back in options[0].
    let options = result.get("options").and_then(|x| x.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| "Yahoo options: options[0] missing".to_string())?;

    let parse_contract = |c: &serde_json::Value, opt_type: &str, underlying: f64| -> OptionContract {
        let strike = c.get("strike").and_then(|x| x.as_f64()).unwrap_or(0.0);
        let itm = match opt_type {
            "CALL" => underlying > strike,
            _      => underlying < strike,
        };
        OptionContract {
            contract_symbol: c.get("contractSymbol").and_then(|x| x.as_str()).unwrap_or("").to_string(),
            option_type: opt_type.to_string(),
            strike,
            last_price: c.get("lastPrice").and_then(|x| x.as_f64()).unwrap_or(0.0),
            bid: c.get("bid").and_then(|x| x.as_f64()).unwrap_or(0.0),
            ask: c.get("ask").and_then(|x| x.as_f64()).unwrap_or(0.0),
            volume: c.get("volume").and_then(|x| x.as_f64()).unwrap_or(0.0),
            open_interest: c.get("openInterest").and_then(|x| x.as_f64()).unwrap_or(0.0),
            implied_volatility: c.get("impliedVolatility").and_then(|x| x.as_f64()).unwrap_or(0.0),
            in_the_money: itm,
        }
    };

    let exp_ts = options.get("expirationDate").and_then(|x| x.as_i64()).unwrap_or(0);
    let expiration = if exp_ts > 0 {
        chrono::DateTime::<chrono::Utc>::from_timestamp(exp_ts, 0)
            .map(|d| d.format("%Y-%m-%d").to_string())
            .unwrap_or_default()
    } else { String::new() };
    let now = chrono::Utc::now().timestamp();
    let days_to_expiry = if exp_ts > now { (exp_ts - now) / 86400 } else { 0 };

    let calls: Vec<OptionContract> = options.get("calls").and_then(|x| x.as_array())
        .map(|arr| arr.iter().map(|c| parse_contract(c, "CALL", underlying_price)).collect())
        .unwrap_or_default();
    let puts: Vec<OptionContract> = options.get("puts").and_then(|x| x.as_array())
        .map(|arr| arr.iter().map(|c| parse_contract(c, "PUT", underlying_price)).collect())
        .unwrap_or_default();

    let note = if expiration_dates.len() > 1 {
        format!("Yahoo returned first of {} expirations; additional dates available",
            expiration_dates.len())
    } else { String::new() };

    Ok(OptionsChainSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: today,
        underlying_price,
        expirations: vec![OptionExpiry { expiration, days_to_expiry, calls, puts }],
        note,
    })
}

// ── ADR-115 Round 8 — IVOL compute (IV Rank / IV Percentile) ─────────────

/// Compute an `IvolSnapshot` from a 52-week history of ATM IV observations
/// plus a current ATM IV reading. The caller is responsible for extracting
/// the ATM IV from an `OptionsChainSnapshot` (or from any other source).
///
/// IV Rank: `(current − 52w low) / (52w high − 52w low) × 100`.
/// IV Percentile: `% of history ≤ current`.
pub fn compute_ivol_snapshot(
    symbol: &str,
    as_of: &str,
    current_atm_iv_pct: f64,
    history: &[IvolObservation],
) -> IvolSnapshot {
    if history.is_empty() {
        return IvolSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            current_atm_iv_pct,
            iv_52w_low_pct: current_atm_iv_pct,
            iv_52w_high_pct: current_atm_iv_pct,
            iv_rank: 50.0,
            iv_percentile: 50.0,
            observation_count: 0,
            history: Vec::new(),
            note: "no IV history — rank/percentile are placeholders until history accumulates".to_string(),
        };
    }
    let mut vals: Vec<f64> = history.iter().map(|o| o.atm_iv_pct).filter(|v| v.is_finite() && *v > 0.0).collect();
    vals.push(current_atm_iv_pct);
    vals.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let lo = vals.first().copied().unwrap_or(current_atm_iv_pct);
    let hi = vals.last().copied().unwrap_or(current_atm_iv_pct);
    let rank = if (hi - lo).abs() > 1e-9 {
        ((current_atm_iv_pct - lo) / (hi - lo)) * 100.0
    } else { 50.0 };
    let le_count = vals.iter().filter(|v| **v <= current_atm_iv_pct).count();
    let pct = (le_count as f64 / vals.len() as f64) * 100.0;

    IvolSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        current_atm_iv_pct,
        iv_52w_low_pct: lo,
        iv_52w_high_pct: hi,
        iv_rank: rank.clamp(0.0, 100.0),
        iv_percentile: pct.clamp(0.0, 100.0),
        observation_count: history.len(),
        history: history.to_vec(),
        note: if history.len() < 20 {
            format!("only {} observations — rank stabilizes around 252", history.len())
        } else { String::new() },
    }
}

// ── ADR-116 Round 9 — SEAG compute (seasonality) ─────────────────────────

/// Compute a `SeasonalitySnapshot` from a chronologically-ordered slice of
/// bars. Builds monthly buckets (Jan..Dec) of year-over-year per-month returns
/// (first bar of month → last bar of month) and day-of-week buckets of daily
/// log-returns. Pure compute, no network.
pub fn compute_seasonality_snapshot(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
) -> SeasonalitySnapshot {
    if bars_oldest_first.len() < 30 {
        return SeasonalitySnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            note: "insufficient bar history (need ≥ 30 bars)".to_string(),
            ..Default::default()
        };
    }

    let px = |b: &HistoricalPriceRow| -> f64 {
        if b.adj_close > 0.0 { b.adj_close } else { b.close }
    };

    // ── Monthly buckets: group bars by YYYY-MM and compute per-(year, month)
    // simple return from first bar to last bar of that month, then aggregate
    // across years into the 12 buckets.
    use std::collections::BTreeMap;
    let mut per_ym: BTreeMap<(i32, u32), (f64, f64)> = BTreeMap::new(); // (year, month) → (first, last)
    let mut years_seen: std::collections::BTreeSet<i32> = std::collections::BTreeSet::new();
    for b in bars_oldest_first {
        if b.date.len() < 10 { continue; }
        let year: i32 = match b.date.get(0..4).and_then(|s| s.parse().ok()) { Some(y) => y, None => continue };
        let month: u32 = match b.date.get(5..7).and_then(|s| s.parse().ok()) { Some(m) => m, None => continue };
        let p = px(b);
        if p <= 0.0 { continue; }
        years_seen.insert(year);
        per_ym.entry((year, month)).and_modify(|e| e.1 = p).or_insert((p, p));
    }

    let month_label = |m: u32| -> &'static str {
        match m {
            1 => "Jan", 2 => "Feb", 3 => "Mar", 4 => "Apr",
            5 => "May", 6 => "Jun", 7 => "Jul", 8 => "Aug",
            9 => "Sep", 10 => "Oct", 11 => "Nov", 12 => "Dec",
            _ => "?",
        }
    };

    let mut months: Vec<SeasonalityMonth> = Vec::new();
    for m in 1u32..=12 {
        let rets: Vec<f64> = per_ym.iter()
            .filter_map(|((_y, mm), (first, last))| {
                if *mm == m && *first > 0.0 { Some((last / first - 1.0) * 100.0) } else { None }
            })
            .collect();
        if rets.is_empty() {
            months.push(SeasonalityMonth { month: m, label: month_label(m).to_string(), ..Default::default() });
            continue;
        }
        let mean = rets.iter().sum::<f64>() / rets.len() as f64;
        let var = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / rets.len() as f64;
        let stdev = var.sqrt();
        let mut sorted = rets.clone();
        sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = sorted[sorted.len() / 2];
        let positive = rets.iter().filter(|r| **r > 0.0).count();
        let best = sorted.last().copied().unwrap_or(0.0);
        let worst = sorted.first().copied().unwrap_or(0.0);
        months.push(SeasonalityMonth {
            month: m,
            label: month_label(m).to_string(),
            avg_return_pct: mean,
            median_return_pct: median,
            stdev_pct: stdev,
            positive_years: positive,
            total_years: rets.len(),
            best_return_pct: best,
            worst_return_pct: worst,
        });
    }

    // ── Day-of-week buckets using log-returns on successive bars.
    let dow_label = |d: u32| -> &'static str {
        match d {
            1 => "Mon", 2 => "Tue", 3 => "Wed", 4 => "Thu",
            5 => "Fri", 6 => "Sat", 7 => "Sun", _ => "?",
        }
    };
    // Zeller-style computation for a YYYY-MM-DD string.
    let dow_of = |date: &str| -> Option<u32> {
        let y: i32 = date.get(0..4)?.parse().ok()?;
        let m: i32 = date.get(5..7)?.parse().ok()?;
        let d: i32 = date.get(8..10)?.parse().ok()?;
        // Zeller's congruence — returns 0=Sat..6=Fri; we remap to 1=Mon..7=Sun.
        let (q, m2, k_year) = if m < 3 { (d, m + 12, y - 1) } else { (d, m, y) };
        let k = k_year % 100;
        let j = k_year / 100;
        let h = (q + (13 * (m2 + 1)) / 5 + k + k / 4 + j / 4 + 5 * j).rem_euclid(7);
        // Zeller h: 0=Sat, 1=Sun, 2=Mon, 3=Tue, 4=Wed, 5=Thu, 6=Fri
        let iso = match h { 0 => 6, 1 => 7, 2 => 1, 3 => 2, 4 => 3, 5 => 4, 6 => 5, _ => 1 };
        Some(iso as u32)
    };

    let mut dow_map: BTreeMap<u32, (f64, usize, usize)> = BTreeMap::new(); // dow → (sum_log_ret, pos_count, total)
    for w in bars_oldest_first.windows(2) {
        let p0 = px(&w[0]);
        let p1 = px(&w[1]);
        if p0 <= 0.0 || p1 <= 0.0 { continue; }
        let r = (p1 / p0).ln();
        if let Some(d) = dow_of(&w[1].date) {
            let entry = dow_map.entry(d).or_insert((0.0, 0, 0));
            entry.0 += r;
            entry.2 += 1;
            if r > 0.0 { entry.1 += 1; }
        }
    }
    let mut dow_out: Vec<SeasonalityDow> = Vec::new();
    for d in 1u32..=5 {
        if let Some((sum, pos, total)) = dow_map.get(&d).cloned() {
            let mean_pct = if total > 0 { (sum / total as f64).exp().ln() * 100.0 } else { 0.0 };
            dow_out.push(SeasonalityDow {
                dow: d,
                label: dow_label(d).to_string(),
                avg_return_pct: mean_pct,
                positive_days: pos,
                total_days: total,
            });
        }
    }

    let mut best_month = String::new();
    let mut worst_month = String::new();
    let mut best_avg = f64::NEG_INFINITY;
    let mut worst_avg = f64::INFINITY;
    for m in &months {
        if m.total_years == 0 { continue; }
        if m.avg_return_pct > best_avg { best_avg = m.avg_return_pct; best_month = m.label.clone(); }
        if m.avg_return_pct < worst_avg { worst_avg = m.avg_return_pct; worst_month = m.label.clone(); }
    }

    SeasonalitySnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        years_covered: years_seen.len(),
        months,
        dow: dow_out,
        best_month,
        worst_month,
        note: String::new(),
    }
}

// ── ADR-116 Round 9 — COR compute (correlation matrix vs peers) ──────────

/// Compute a pairwise correlation matrix for a subject symbol against a set
/// of peer bar series over a rolling window of `window_days`. Uses Pearson
/// correlation on daily log-returns intersected by date, skipping peers with
/// fewer than 30 overlapping observations. Pure compute.
pub fn compute_correlation_matrix(
    symbol: &str,
    as_of: &str,
    window_days: usize,
    subject_bars: &[HistoricalPriceRow],
    peer_series: &[(String, Vec<HistoricalPriceRow>)],
) -> CorrelationMatrix {
    let px = |b: &HistoricalPriceRow| -> f64 {
        if b.adj_close > 0.0 { b.adj_close } else { b.close }
    };
    // Truncate subject to the most recent `window_days` bars (plus one anchor).
    let take = window_days.saturating_add(1).min(subject_bars.len());
    let subject_slice = &subject_bars[subject_bars.len() - take..];
    if subject_slice.len() < 31 {
        return CorrelationMatrix {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            window_days,
            note: "insufficient subject bar history (need ≥ 31)".to_string(),
            ..Default::default()
        };
    }

    // Build date→logret map for subject.
    use std::collections::HashMap;
    let mut sub_map: HashMap<String, f64> = HashMap::new();
    for w in subject_slice.windows(2) {
        let p0 = px(&w[0]);
        let p1 = px(&w[1]);
        if p0 > 0.0 && p1 > 0.0 {
            sub_map.insert(w[1].date.clone(), (p1 / p0).ln());
        }
    }

    let mut cells: Vec<CorrelationCell> = Vec::new();
    for (peer_sym, peer_bars) in peer_series {
        if peer_bars.len() < 31 { continue; }
        let ptake = window_days.saturating_add(1).min(peer_bars.len());
        let peer_slice = &peer_bars[peer_bars.len() - ptake..];
        // Build peer logret and intersect dates.
        let mut paired: Vec<(f64, f64)> = Vec::new();
        for w in peer_slice.windows(2) {
            let p0 = px(&w[0]);
            let p1 = px(&w[1]);
            if p0 <= 0.0 || p1 <= 0.0 { continue; }
            if let Some(s) = sub_map.get(&w[1].date) {
                paired.push((*s, (p1 / p0).ln()));
            }
        }
        if paired.len() < 30 { continue; }
        let n = paired.len() as f64;
        let mean_s: f64 = paired.iter().map(|(s, _)| *s).sum::<f64>() / n;
        let mean_p: f64 = paired.iter().map(|(_, p)| *p).sum::<f64>() / n;
        let mut cov = 0.0;
        let mut var_s = 0.0;
        let mut var_p = 0.0;
        for (s, p) in &paired {
            let ds = s - mean_s;
            let dp = p - mean_p;
            cov += ds * dp;
            var_s += ds * ds;
            var_p += dp * dp;
        }
        let denom = (var_s * var_p).sqrt();
        let rho = if denom > 1e-12 { cov / denom } else { 0.0 };
        let beta = if var_p > 1e-12 { cov / var_p } else { 0.0 };
        cells.push(CorrelationCell {
            peer_symbol: peer_sym.to_uppercase(),
            correlation: rho.clamp(-1.0, 1.0),
            n_observations: paired.len(),
            beta_vs_peer: beta,
        });
    }

    if cells.is_empty() {
        return CorrelationMatrix {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            window_days,
            note: "no peer pairs with ≥ 30 overlapping observations".to_string(),
            ..Default::default()
        };
    }
    let mean_corr = cells.iter().map(|c| c.correlation.abs()).sum::<f64>() / cells.len() as f64;
    let mut highest_sym = String::new();
    let mut lowest_sym = String::new();
    let mut hi = f64::NEG_INFINITY;
    let mut lo = f64::INFINITY;
    for c in &cells {
        if c.correlation > hi { hi = c.correlation; highest_sym = c.peer_symbol.clone(); }
        if c.correlation < lo { lo = c.correlation; lowest_sym = c.peer_symbol.clone(); }
    }

    CorrelationMatrix {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        window_days,
        cells,
        mean_correlation: mean_corr,
        highest_corr_symbol: highest_sym,
        lowest_corr_symbol: lowest_sym,
        note: String::new(),
    }
}

// ── ADR-116 Round 9 — TRA compute (total return = price + dividends) ────

/// Compute a `TotalReturnSnapshot` by combining HP price returns with the
/// sum of cash dividends paid over the same window. Pure compute; inputs are
/// already-cached bars and dividend records.
pub fn compute_total_return_snapshot(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
    dividends: &[DividendRecord],
) -> TotalReturnSnapshot {
    if bars_oldest_first.len() < 2 {
        return TotalReturnSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            note: "insufficient bar history (need ≥ 2 bars)".to_string(),
            ..Default::default()
        };
    }
    let n = bars_oldest_first.len();
    let last_close = bars_oldest_first[n - 1].close;
    let last_date = bars_oldest_first[n - 1].date.clone();

    let px = |i: usize| -> f64 {
        let b = &bars_oldest_first[i];
        if b.adj_close > 0.0 { b.adj_close } else { b.close }
    };
    let last_px = px(n - 1);

    // Trailing 12 month dividends by ex_date cutoff.
    let cutoff_ttm = {
        // Naive 12-month cutoff: subtract one from the year component.
        let y: i32 = last_date.get(0..4).and_then(|s| s.parse().ok()).unwrap_or(0);
        let m = last_date.get(5..7).unwrap_or("01");
        let d = last_date.get(8..10).unwrap_or("01");
        format!("{:04}-{}-{}", y - 1, m, d)
    };
    let ttm_divs: f64 = dividends.iter()
        .filter(|d| d.ex_date.as_str() > cutoff_ttm.as_str() && d.ex_date.as_str() <= last_date.as_str())
        .map(|d| d.amount)
        .sum();
    let ttm_yield = if last_close > 0.0 { ttm_divs / last_close * 100.0 } else { 0.0 };

    let mut windows: Vec<TotalReturnWindow> = Vec::new();
    let push_window = |windows: &mut Vec<TotalReturnWindow>, label: &str, start_idx: usize, trading_days: usize| {
        if start_idx >= n - 1 { return; }
        let start_px = px(start_idx);
        if start_px <= 0.0 { return; }
        let start_date = bars_oldest_first[start_idx].date.clone();
        let price_ret = (last_px / start_px - 1.0) * 100.0;
        let window_divs: f64 = dividends.iter()
            .filter(|d| d.ex_date.as_str() > start_date.as_str() && d.ex_date.as_str() <= last_date.as_str())
            .map(|d| d.amount)
            .sum();
        let n_divs = dividends.iter()
            .filter(|d| d.ex_date.as_str() > start_date.as_str() && d.ex_date.as_str() <= last_date.as_str())
            .count();
        let div_yield = if start_px > 0.0 { window_divs / start_px * 100.0 } else { 0.0 };
        let total = price_ret + div_yield;
        let annualized = if trading_days >= 252 {
            let years = trading_days as f64 / 252.0;
            (((total / 100.0) + 1.0).powf(1.0 / years) - 1.0) * 100.0
        } else { total };
        windows.push(TotalReturnWindow {
            label: label.to_string(),
            trading_days,
            price_return_pct: price_ret,
            dividend_yield_pct: div_yield,
            total_return_pct: total,
            annualized_pct: annualized,
            dividends_paid: window_divs,
            n_dividends: n_divs,
        });
    };

    for (label, days) in &[("1M", 21), ("3M", 63), ("6M", 126), ("1Y", 252), ("3Y", 756), ("5Y", 1260)] {
        if n > *days {
            push_window(&mut windows, label, n - 1 - days, *days);
        }
    }
    // YTD
    let year_prefix = as_of.get(..4).unwrap_or("");
    if !year_prefix.is_empty() {
        if let Some(ytd_start) = bars_oldest_first.iter().position(|b| b.date.starts_with(year_prefix)) {
            push_window(&mut windows, "YTD", ytd_start, n - ytd_start);
        }
    }

    TotalReturnSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        last_close,
        trailing_12m_dividends: ttm_divs,
        trailing_12m_yield_pct: ttm_yield,
        windows,
        note: String::new(),
    }
}

// ── ADR-116 Round 9 — TECH compute (technical indicators) ────────────────

/// Compute standard technical indicators (RSI, MACD, Bollinger, ATR, ADX,
/// Stochastic) from a chronologically-ordered slice of bars. Pure compute.
pub fn compute_technical_indicators(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
) -> TechnicalSnapshot {
    if bars_oldest_first.len() < 35 {
        return TechnicalSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            note: "insufficient bar history (need ≥ 35 bars)".to_string(),
            ..Default::default()
        };
    }
    let n = bars_oldest_first.len();
    let closes: Vec<f64> = bars_oldest_first.iter().map(|b| if b.adj_close > 0.0 { b.adj_close } else { b.close }).collect();
    let highs: Vec<f64> = bars_oldest_first.iter().map(|b| b.high.max(b.close)).collect();
    let lows: Vec<f64> = bars_oldest_first.iter().map(|b| b.low.min(b.close)).collect();
    let last_close = closes[n - 1];

    let mut out: Vec<TechnicalIndicator> = Vec::new();

    // RSI(14) — Wilder's smoothing.
    if n >= 15 {
        let mut gains: Vec<f64> = Vec::with_capacity(n - 1);
        let mut losses: Vec<f64> = Vec::with_capacity(n - 1);
        for i in 1..n {
            let diff = closes[i] - closes[i - 1];
            gains.push(if diff > 0.0 { diff } else { 0.0 });
            losses.push(if diff < 0.0 { -diff } else { 0.0 });
        }
        let mut avg_gain: f64 = gains[..14].iter().sum::<f64>() / 14.0;
        let mut avg_loss: f64 = losses[..14].iter().sum::<f64>() / 14.0;
        for i in 14..gains.len() {
            avg_gain = (avg_gain * 13.0 + gains[i]) / 14.0;
            avg_loss = (avg_loss * 13.0 + losses[i]) / 14.0;
        }
        let rs = if avg_loss > 1e-12 { avg_gain / avg_loss } else { f64::INFINITY };
        let rsi = if rs.is_infinite() { 100.0 } else { 100.0 - 100.0 / (1.0 + rs) };
        let signal = if rsi >= 70.0 { "overbought" }
                     else if rsi <= 30.0 { "oversold" }
                     else if rsi >= 55.0 { "bullish" }
                     else if rsi <= 45.0 { "bearish" }
                     else { "neutral" };
        out.push(TechnicalIndicator {
            name: "RSI(14)".to_string(),
            value: rsi,
            value_secondary: 0.0,
            value_tertiary: 0.0,
            signal: signal.to_string(),
            note: String::new(),
        });
    }

    // MACD(12,26,9) — EMA crossover.
    if n >= 35 {
        let ema = |period: usize, data: &[f64]| -> Vec<f64> {
            let k = 2.0 / (period as f64 + 1.0);
            let mut out = Vec::with_capacity(data.len());
            let mut prev = data[0];
            out.push(prev);
            for v in &data[1..] {
                prev = v * k + prev * (1.0 - k);
                out.push(prev);
            }
            out
        };
        let ema12 = ema(12, &closes);
        let ema26 = ema(26, &closes);
        let macd_line: Vec<f64> = ema12.iter().zip(ema26.iter()).map(|(a, b)| a - b).collect();
        let signal_line = ema(9, &macd_line);
        let macd = *macd_line.last().unwrap_or(&0.0);
        let sig = *signal_line.last().unwrap_or(&0.0);
        let hist = macd - sig;
        let signal = if hist > 0.0 { "bullish" } else if hist < 0.0 { "bearish" } else { "neutral" };
        out.push(TechnicalIndicator {
            name: "MACD(12,26,9)".to_string(),
            value: hist,
            value_secondary: macd,
            value_tertiary: sig,
            signal: signal.to_string(),
            note: format!("MACD={:.3} Signal={:.3}", macd, sig),
        });
    }

    // Bollinger Bands (20, 2σ).
    if n >= 20 {
        let slice = &closes[n - 20..];
        let mean: f64 = slice.iter().sum::<f64>() / 20.0;
        let var: f64 = slice.iter().map(|c| (c - mean).powi(2)).sum::<f64>() / 20.0;
        let sd = var.sqrt();
        let upper = mean + 2.0 * sd;
        let lower = mean - 2.0 * sd;
        let bandwidth_pct = if mean > 0.0 { (upper - lower) / mean * 100.0 } else { 0.0 };
        let pct_b = if (upper - lower).abs() > 1e-9 { (last_close - lower) / (upper - lower) * 100.0 } else { 50.0 };
        let signal = if pct_b >= 100.0 { "overbought" }
                     else if pct_b <= 0.0 { "oversold" }
                     else if pct_b >= 80.0 { "bullish" }
                     else if pct_b <= 20.0 { "bearish" }
                     else { "neutral" };
        out.push(TechnicalIndicator {
            name: "BB(20,2)".to_string(),
            value: pct_b,
            value_secondary: upper,
            value_tertiary: lower,
            signal: signal.to_string(),
            note: format!("mid={:.2} bw={:.2}%", mean, bandwidth_pct),
        });
    }

    // ATR(14) — Wilder.
    if n >= 15 {
        let mut tr: Vec<f64> = Vec::with_capacity(n - 1);
        for i in 1..n {
            let hl = highs[i] - lows[i];
            let hc = (highs[i] - closes[i - 1]).abs();
            let lc = (lows[i] - closes[i - 1]).abs();
            tr.push(hl.max(hc).max(lc));
        }
        let mut atr: f64 = tr[..14].iter().sum::<f64>() / 14.0;
        for v in &tr[14..] {
            atr = (atr * 13.0 + v) / 14.0;
        }
        let atr_pct = if last_close > 0.0 { atr / last_close * 100.0 } else { 0.0 };
        out.push(TechnicalIndicator {
            name: "ATR(14)".to_string(),
            value: atr,
            value_secondary: atr_pct,
            value_tertiary: 0.0,
            signal: "neutral".to_string(),
            note: format!("{:.2}% of close", atr_pct),
        });
    }

    // ADX(14) — Wilder directional movement.
    if n >= 28 {
        let mut plus_dm: Vec<f64> = Vec::with_capacity(n - 1);
        let mut minus_dm: Vec<f64> = Vec::with_capacity(n - 1);
        let mut tr: Vec<f64> = Vec::with_capacity(n - 1);
        for i in 1..n {
            let up = highs[i] - highs[i - 1];
            let down = lows[i - 1] - lows[i];
            plus_dm.push(if up > down && up > 0.0 { up } else { 0.0 });
            minus_dm.push(if down > up && down > 0.0 { down } else { 0.0 });
            let hl = highs[i] - lows[i];
            let hc = (highs[i] - closes[i - 1]).abs();
            let lc = (lows[i] - closes[i - 1]).abs();
            tr.push(hl.max(hc).max(lc));
        }
        // Wilder smoothing (14).
        let mut pdm: f64 = plus_dm[..14].iter().sum::<f64>();
        let mut mdm: f64 = minus_dm[..14].iter().sum::<f64>();
        let mut trs: f64 = tr[..14].iter().sum::<f64>();
        let mut dx_hist: Vec<f64> = Vec::new();
        for i in 14..plus_dm.len() {
            pdm = pdm - pdm / 14.0 + plus_dm[i];
            mdm = mdm - mdm / 14.0 + minus_dm[i];
            trs = trs - trs / 14.0 + tr[i];
            let plus_di = if trs > 1e-12 { pdm / trs * 100.0 } else { 0.0 };
            let minus_di = if trs > 1e-12 { mdm / trs * 100.0 } else { 0.0 };
            let sum = plus_di + minus_di;
            let dx = if sum > 1e-12 { ((plus_di - minus_di).abs() / sum) * 100.0 } else { 0.0 };
            dx_hist.push(dx);
        }
        if dx_hist.len() >= 14 {
            let mut adx: f64 = dx_hist[..14].iter().sum::<f64>() / 14.0;
            for v in &dx_hist[14..] {
                adx = (adx * 13.0 + v) / 14.0;
            }
            let plus_di = if trs > 1e-12 { pdm / trs * 100.0 } else { 0.0 };
            let minus_di = if trs > 1e-12 { mdm / trs * 100.0 } else { 0.0 };
            let signal = if adx >= 25.0 {
                if plus_di > minus_di { "bullish" } else { "bearish" }
            } else { "neutral" };
            out.push(TechnicalIndicator {
                name: "ADX(14)".to_string(),
                value: adx,
                value_secondary: plus_di,
                value_tertiary: minus_di,
                signal: signal.to_string(),
                note: format!("+DI={:.1} −DI={:.1}", plus_di, minus_di),
            });
        }
    }

    // Stochastic %K(14), %D(3).
    if n >= 17 {
        let mut k_series: Vec<f64> = Vec::new();
        for i in 13..n {
            let window_high = highs[i - 13..=i].iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let window_low  = lows[i - 13..=i].iter().cloned().fold(f64::INFINITY, f64::min);
            let denom = window_high - window_low;
            let k = if denom.abs() > 1e-12 { (closes[i] - window_low) / denom * 100.0 } else { 50.0 };
            k_series.push(k);
        }
        let k_last = *k_series.last().unwrap_or(&50.0);
        let d_last = if k_series.len() >= 3 {
            k_series[k_series.len() - 3..].iter().sum::<f64>() / 3.0
        } else { k_last };
        let signal = if k_last >= 80.0 { "overbought" }
                     else if k_last <= 20.0 { "oversold" }
                     else if k_last > d_last { "bullish" }
                     else if k_last < d_last { "bearish" }
                     else { "neutral" };
        out.push(TechnicalIndicator {
            name: "Stoch(14,3)".to_string(),
            value: k_last,
            value_secondary: d_last,
            value_tertiary: 0.0,
            signal: signal.to_string(),
            note: format!("%K={:.1} %D={:.1}", k_last, d_last),
        });
    }

    // Trend synthesis — count bullish/bearish across tradeable indicators.
    let mut bull = 0usize;
    let mut bear = 0usize;
    for ind in &out {
        match ind.signal.as_str() {
            "bullish" | "overbought" => bull += 1,
            "bearish" | "oversold"   => bear += 1,
            _ => {}
        }
    }
    let trend_summary = if bull > bear + 1 { "bullish composite".to_string() }
                        else if bear > bull + 1 { "bearish composite".to_string() }
                        else { "mixed / neutral composite".to_string() };

    TechnicalSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        last_close,
        indicators: out,
        trend_summary,
        note: String::new(),
    }
}

// ── ADR-116 Round 9 — SKEW compute (volatility smile/skew) ───────────────

/// Compute a `VolatilitySkew` snapshot from a cached options chain. For each
/// expiry, walk the strike ladder and emit a `SkewPoint` combining call & put
/// IV at that strike; compute ATM IV from the nearest-to-money strike, and
/// approximate a 25-delta put-call skew using ±10% OTM contracts.
pub fn compute_volatility_skew(
    symbol: &str,
    as_of: &str,
    chain: &OptionsChainSnapshot,
) -> VolatilitySkew {
    if chain.expirations.is_empty() || chain.underlying_price <= 0.0 {
        return VolatilitySkew {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            underlying_price: chain.underlying_price,
            note: "no expirations in options chain".to_string(),
            ..Default::default()
        };
    }

    let u = chain.underlying_price;
    let mut out_expiries: Vec<SkewExpiry> = Vec::new();

    for ex in &chain.expirations {
        // Merge calls + puts by strike.
        use std::collections::BTreeMap;
        let mut map: BTreeMap<i64, (Option<f64>, Option<f64>)> = BTreeMap::new(); // key = strike×100
        for c in &ex.calls {
            if c.implied_volatility <= 0.0 { continue; }
            let k = (c.strike * 100.0).round() as i64;
            map.entry(k).and_modify(|e| e.0 = Some(c.implied_volatility)).or_insert((Some(c.implied_volatility), None));
        }
        for p in &ex.puts {
            if p.implied_volatility <= 0.0 { continue; }
            let k = (p.strike * 100.0).round() as i64;
            map.entry(k).and_modify(|e| e.1 = Some(p.implied_volatility)).or_insert((None, Some(p.implied_volatility)));
        }
        let mut points: Vec<SkewPoint> = Vec::new();
        for (k, (cv, pv)) in &map {
            let strike = (*k as f64) / 100.0;
            let moneyness = (strike / u - 1.0) * 100.0;
            let call_iv = cv.map(|v| v * 100.0).unwrap_or(0.0);
            let put_iv = pv.map(|v| v * 100.0).unwrap_or(0.0);
            let combined = match (cv, pv) {
                (Some(a), Some(b)) => (a + b) / 2.0 * 100.0,
                (Some(a), None)    => a * 100.0,
                (None, Some(b))    => b * 100.0,
                (None, None)       => 0.0,
            };
            points.push(SkewPoint {
                strike,
                moneyness_pct: moneyness,
                call_iv_pct: call_iv,
                put_iv_pct: put_iv,
                combined_iv_pct: combined,
            });
        }

        if points.is_empty() {
            out_expiries.push(SkewExpiry {
                expiration: ex.expiration.clone(),
                days_to_expiry: ex.days_to_expiry,
                atm_iv_pct: 0.0,
                points,
                put_call_skew_25d_pct: 0.0,
                term_note: "no IV-populated strikes".to_string(),
            });
            continue;
        }

        // ATM IV: find strike closest to underlying.
        let mut atm_idx = 0usize;
        let mut best_dist = f64::INFINITY;
        for (i, p) in points.iter().enumerate() {
            let d = (p.strike - u).abs();
            if d < best_dist { best_dist = d; atm_idx = i; }
        }
        let atm_iv = points[atm_idx].combined_iv_pct;

        // ±10% OTM skew proxy.
        let target_otm_call = u * 1.10;
        let target_otm_put  = u * 0.90;
        let mut otm_call_iv = 0.0;
        let mut otm_put_iv = 0.0;
        let mut best_c = f64::INFINITY;
        let mut best_p = f64::INFINITY;
        for p in &points {
            let dc = (p.strike - target_otm_call).abs();
            let dp = (p.strike - target_otm_put).abs();
            if dc < best_c && p.call_iv_pct > 0.0 { best_c = dc; otm_call_iv = p.call_iv_pct; }
            if dp < best_p && p.put_iv_pct > 0.0 { best_p = dp; otm_put_iv = p.put_iv_pct; }
        }
        let skew = otm_put_iv - otm_call_iv;

        out_expiries.push(SkewExpiry {
            expiration: ex.expiration.clone(),
            days_to_expiry: ex.days_to_expiry,
            atm_iv_pct: atm_iv,
            points,
            put_call_skew_25d_pct: skew,
            term_note: String::new(),
        });
    }

    VolatilitySkew {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        underlying_price: u,
        expiries: out_expiries,
        note: String::new(),
    }
}

// ── ADR-117 Round 10 — LEV compute (leverage & coverage ratios) ─────────────

/// Compute a `LeverageSnapshot` from cached financial statements and the
/// Fundamentals row. Pulls trailing-12-month EBITDA + interest expense from
/// quarterly income statements, total debt / equity / cash from the most
/// recent annual balance sheet, and produces a standard battery of ratios.
pub fn compute_leverage_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
    total_debt_fund: f64,
    cash_fund: f64,
) -> LeverageSnapshot {
    // Prefer the latest annual balance sheet; fall back to the most recent quarter.
    let bal = statements.balance_annual.first()
        .or_else(|| statements.balance_quarterly.first());

    let total_debt = bal.map(|b| b.total_debt).filter(|v| *v > 0.0).unwrap_or(total_debt_fund);
    let cash = bal.map(|b| b.cash_and_equiv).filter(|v| *v > 0.0).unwrap_or(cash_fund);
    let net_debt = (total_debt - cash).max(0.0);
    let total_equity = bal.map(|b| b.total_equity).unwrap_or(0.0);

    // TTM roll-ups from quarterly income statements (last 4 quarters).
    let q = &statements.income_quarterly;
    let take = q.iter().take(4);
    let ebitda_ttm: f64 = take.clone().map(|i| i.ebitda).sum();
    let interest_ttm: f64 = take.clone().map(|i| i.interest_expense.abs()).sum();
    let op_inc_ttm: f64 = take.clone().map(|i| i.operating_income).sum();

    let cur_assets = bal.map(|b| b.total_current_assets).unwrap_or(0.0);
    let cur_liab = bal.map(|b| b.total_current_liabilities).unwrap_or(0.0);
    let inventory = bal.map(|b| b.inventory).unwrap_or(0.0);

    let mut ratios: Vec<LeverageRatio> = Vec::new();

    // Debt / EBITDA
    if ebitda_ttm > 0.0 && total_debt > 0.0 {
        let v = total_debt / ebitda_ttm;
        let sig = if v < 2.5 { "HEALTHY" } else if v < 4.0 { "ELEVATED" } else { "STRETCHED" };
        ratios.push(LeverageRatio {
            name: "Debt / EBITDA".into(), value: v, peer_median: 0.0,
            signal: sig.into(),
            note: "lower is safer; >4× typically flags high leverage".into(),
        });
    }

    // Net Debt / EBITDA
    if ebitda_ttm > 0.0 {
        let v = net_debt / ebitda_ttm;
        let sig = if v < 2.0 { "HEALTHY" } else if v < 3.5 { "ELEVATED" } else { "STRETCHED" };
        ratios.push(LeverageRatio {
            name: "Net Debt / EBITDA".into(), value: v, peer_median: 0.0,
            signal: sig.into(), note: "net of cash; negative when cash > debt".into(),
        });
    }

    // Debt / Equity
    if total_equity > 0.0 && total_debt > 0.0 {
        let v = total_debt / total_equity;
        let sig = if v < 1.0 { "HEALTHY" } else if v < 2.0 { "ELEVATED" } else { "STRETCHED" };
        ratios.push(LeverageRatio {
            name: "Debt / Equity".into(), value: v, peer_median: 0.0,
            signal: sig.into(), note: "gearing ratio; varies by sector".into(),
        });
    }

    // Interest Coverage (EBIT / Interest)
    if interest_ttm > 0.0 {
        let v = op_inc_ttm / interest_ttm;
        let sig = if v >= 5.0 { "HEALTHY" } else if v >= 2.0 { "ELEVATED" } else { "STRETCHED" };
        ratios.push(LeverageRatio {
            name: "Interest Coverage".into(), value: v, peer_median: 0.0,
            signal: sig.into(),
            note: "EBIT / interest expense; higher is safer; <2× distress signal".into(),
        });
    }

    // Current Ratio
    if cur_liab > 0.0 && cur_assets > 0.0 {
        let v = cur_assets / cur_liab;
        let sig = if v >= 1.5 { "HEALTHY" } else if v >= 1.0 { "ELEVATED" } else { "STRETCHED" };
        ratios.push(LeverageRatio {
            name: "Current Ratio".into(), value: v, peer_median: 0.0,
            signal: sig.into(),
            note: "short-term liquidity; <1 flags near-term squeeze".into(),
        });
    }

    // Quick Ratio
    if cur_liab > 0.0 && cur_assets > 0.0 {
        let v = (cur_assets - inventory) / cur_liab;
        let sig = if v >= 1.0 { "HEALTHY" } else if v >= 0.7 { "ELEVATED" } else { "STRETCHED" };
        ratios.push(LeverageRatio {
            name: "Quick Ratio".into(), value: v, peer_median: 0.0,
            signal: sig.into(),
            note: "excludes inventory; more conservative than current ratio".into(),
        });
    }

    // Solvency summary: count HEALTHY vs STRETCHED signals.
    let n_health = ratios.iter().filter(|r| r.signal == "HEALTHY").count();
    let n_stretch = ratios.iter().filter(|r| r.signal == "STRETCHED").count();
    let solvency_summary = if ratios.is_empty() {
        "insufficient data — run FA + EVSCRAPE first".to_string()
    } else if n_stretch >= 2 {
        format!("STRETCHED — {}/{} ratios flagged", n_stretch, ratios.len())
    } else if n_health >= ratios.len() / 2 + 1 {
        format!("HEALTHY — {}/{} ratios in safe zone", n_health, ratios.len())
    } else {
        "MIXED — some pressure points but no widespread stress".to_string()
    };

    let note = if ratios.is_empty() {
        "no cached financial statements — run FA".to_string()
    } else {
        String::new()
    };

    LeverageSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        total_debt,
        net_debt,
        ebitda_ttm,
        interest_expense_ttm: interest_ttm,
        total_equity,
        ratios,
        solvency_summary,
        note,
    }
}

// ── ADR-117 Round 10 — ACRL compute (earnings quality / accruals) ───────────

/// Compute an `AccrualsSnapshot` from cached financial statements. Walks the
/// last 4 quarterly income + cash-flow pairs, producing an FCF/NI ratio per
/// period plus a TTM roll-up and trend label.
pub fn compute_accruals_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
) -> AccrualsSnapshot {
    let mut periods: Vec<AccrualPeriod> = Vec::new();

    // Match income rows to cashflow rows by date. Keep order as-provided (newest first).
    for inc in statements.income_quarterly.iter().take(8) {
        let cf = statements.cashflow_quarterly.iter().find(|c| c.date == inc.date);
        let ni = inc.net_income;
        let fcf = cf.map(|c| c.free_cash_flow).unwrap_or(0.0);
        if ni == 0.0 && fcf == 0.0 { continue; }
        let ratio = if ni != 0.0 { fcf / ni } else { 0.0 };
        let conv_pct = ratio * 100.0;
        let accruals = ni - fcf;
        let quality_label = if ni <= 0.0 {
            "NEGATIVE_NI".to_string()
        } else if conv_pct >= 90.0 {
            "HIGH".to_string()
        } else if conv_pct >= 60.0 {
            "MEDIUM".to_string()
        } else {
            "LOW".to_string()
        };
        periods.push(AccrualPeriod {
            period: inc.period.clone(),
            date: inc.date.clone(),
            net_income: ni,
            free_cash_flow: fcf,
            fcf_to_ni_ratio: ratio,
            cash_conversion_pct: conv_pct,
            accruals,
            quality_label,
        });
    }

    // TTM roll-up from the last 4 quarters.
    let ttm_ni: f64 = periods.iter().take(4).map(|p| p.net_income).sum();
    let ttm_fcf: f64 = periods.iter().take(4).map(|p| p.free_cash_flow).sum();
    let ttm_conv_pct = if ttm_ni != 0.0 { ttm_fcf / ttm_ni * 100.0 } else { 0.0 };

    let avg_conv_pct: f64 = if !periods.is_empty() {
        periods.iter().map(|p| p.cash_conversion_pct).sum::<f64>() / periods.len() as f64
    } else { 0.0 };

    // Trend: compare recent-2 average vs older-2 average.
    let trend_label = if periods.len() < 4 {
        "INSUFFICIENT".to_string()
    } else {
        let recent: f64 = periods.iter().take(2).map(|p| p.cash_conversion_pct).sum::<f64>() / 2.0;
        let older: f64 = periods.iter().skip(2).take(2).map(|p| p.cash_conversion_pct).sum::<f64>() / 2.0;
        let delta = recent - older;
        if delta.abs() < 5.0 { "STABLE".to_string() }
        else if delta > 0.0 { "IMPROVING".to_string() }
        else { "DETERIORATING".to_string() }
    };

    let note = if periods.is_empty() {
        "no cached quarterly statements — run FA".to_string()
    } else {
        String::new()
    };

    AccrualsSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        ttm_net_income: ttm_ni,
        ttm_free_cash_flow: ttm_fcf,
        ttm_cash_conversion_pct: ttm_conv_pct,
        avg_cash_conversion_pct: avg_conv_pct,
        periods,
        trend_label,
        note,
    }
}

// ── ADR-117 Round 10 — RVOL compute (realized volatility cone) ──────────────

/// Compute a `RealizedVolSnapshot` from oldest-first daily bars. Produces
/// rolling 20d / 60d / 120d / 252d realized volatility (annualized stdev of
/// daily log-returns × √252) plus a cone percentile for each window
/// (where does today's RV rank against the full history of that window?),
/// and — when `current_atm_iv_pct > 0` — an IV / RV gap and ratio.
pub fn compute_realized_vol_snapshot(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
    current_atm_iv_pct: f64,
) -> RealizedVolSnapshot {
    if bars_oldest_first.len() < 25 {
        return RealizedVolSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥25 daily bars; run HP first".into(),
            ..Default::default()
        };
    }

    let last_close = bars_oldest_first.last().map(|b| b.close).unwrap_or(0.0);

    let mut log_returns: Vec<f64> = Vec::with_capacity(bars_oldest_first.len() - 1);
    for w in bars_oldest_first.windows(2) {
        if w[0].close > 0.0 && w[1].close > 0.0 {
            log_returns.push((w[1].close / w[0].close).ln());
        }
    }

    let stdev = |xs: &[f64]| -> f64 {
        if xs.len() < 2 { return 0.0; }
        let mean = xs.iter().sum::<f64>() / xs.len() as f64;
        let var: f64 = xs.iter().map(|x| (x - mean).powi(2)).sum::<f64>() / (xs.len() as f64 - 1.0);
        var.sqrt()
    };
    let ann_vol_pct = |xs: &[f64]| -> f64 { stdev(xs) * (252.0_f64).sqrt() * 100.0 };

    let rolling_vols = |window: usize| -> (f64, Vec<f64>) {
        if log_returns.len() < window {
            return (0.0, Vec::new());
        }
        let mut series: Vec<f64> = Vec::new();
        for i in window..=log_returns.len() {
            let slice = &log_returns[i - window..i];
            series.push(ann_vol_pct(slice));
        }
        let latest = *series.last().unwrap_or(&0.0);
        (latest, series)
    };

    let specs = [
        ("20d", 20usize),
        ("60d", 60usize),
        ("120d", 120usize),
        ("252d", 252usize),
    ];
    let mut windows: Vec<RealizedVolWindow> = Vec::new();
    let mut rv_20d = 0.0;
    for (label, n) in specs.iter() {
        let (latest, series) = rolling_vols(*n);
        if series.is_empty() { continue; }
        if *label == "20d" { rv_20d = latest; }
        // Percentile rank of `latest` within its own rolling history.
        let count_below = series.iter().filter(|v| **v < latest).count();
        let pct = (count_below as f64 / series.len() as f64) * 100.0;
        windows.push(RealizedVolWindow {
            label: (*label).to_string(),
            trading_days: *n,
            realized_vol_pct: latest,
            percentile: pct,
            n_observations: series.len(),
        });
    }

    let (iv_rv_gap, iv_rv_ratio, regime_label) = if current_atm_iv_pct > 0.0 && rv_20d > 0.0 {
        let gap = current_atm_iv_pct - rv_20d;
        let ratio = current_atm_iv_pct / rv_20d;
        let label = if ratio < 0.95 { "CHEAP_IV".to_string() }
                    else if ratio > 1.15 { "RICH_IV".to_string() }
                    else { "FAIR_IV".to_string() };
        (gap, ratio, label)
    } else if rv_20d > 0.0 {
        (0.0, 0.0, "NO_IV_REFERENCE".to_string())
    } else {
        (0.0, 0.0, "INSUFFICIENT_DATA".to_string())
    };

    let note = if windows.is_empty() {
        "need more bars for rolling windows".to_string()
    } else {
        String::new()
    };

    RealizedVolSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        last_close,
        current_atm_iv_pct,
        iv_rv_gap_pct: iv_rv_gap,
        iv_rv_ratio,
        windows,
        regime_label,
        note,
    }
}

// ── ADR-117 Round 10 — FCFY compute (FCF yield + dividend coverage) ─────────

/// Compute an `FcfYieldSnapshot` from cached financial statements + market cap.
/// Builds per-annual FCF yield / dividend coverage rows, rolls TTM from the last
/// 4 quarterly cash flow statements, computes a 5-year FCF CAGR when enough
/// annual rows exist, and emits a dividend-sustainability label.
pub fn compute_fcf_yield_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
    market_cap: f64,
    stock_price: f64,
) -> FcfYieldSnapshot {
    let mut periods: Vec<FcfYieldPeriod> = Vec::new();

    for cf in statements.cashflow_annual.iter().take(5) {
        let ni = statements.income_annual.iter()
            .find(|i| i.date == cf.date).map(|i| i.net_income).unwrap_or(0.0);
        let div = cf.dividends_paid.abs();
        let payout_fcf = if cf.free_cash_flow > 0.0 { div / cf.free_cash_flow * 100.0 } else { 0.0 };
        let payout_ni = if ni > 0.0 { div / ni * 100.0 } else { 0.0 };
        let yield_pct = if market_cap > 0.0 { cf.free_cash_flow / market_cap * 100.0 } else { 0.0 };
        periods.push(FcfYieldPeriod {
            period: cf.period.clone(),
            date: cf.date.clone(),
            free_cash_flow: cf.free_cash_flow,
            dividends_paid: div,
            payout_from_fcf_pct: payout_fcf,
            payout_from_ni_pct: payout_ni,
            fcf_yield_pct: yield_pct,
        });
    }

    // TTM roll-up from the last 4 quarterly cash flow statements.
    let q_cf = &statements.cashflow_quarterly;
    let ttm_fcf: f64 = q_cf.iter().take(4).map(|c| c.free_cash_flow).sum();
    let ttm_div: f64 = q_cf.iter().take(4).map(|c| c.dividends_paid.abs()).sum();
    let ttm_ni: f64 = statements.income_quarterly.iter().take(4).map(|i| i.net_income).sum();
    let ttm_fcf_yield = if market_cap > 0.0 { ttm_fcf / market_cap * 100.0 } else { 0.0 };
    let ttm_div_yield = if market_cap > 0.0 { ttm_div / market_cap * 100.0 } else { 0.0 };
    let ttm_payout_fcf = if ttm_fcf > 0.0 { ttm_div / ttm_fcf * 100.0 } else { 0.0 };
    let ttm_payout_ni = if ttm_ni > 0.0 { ttm_div / ttm_ni * 100.0 } else { 0.0 };

    // 5-year FCF CAGR (oldest → newest) when we have ≥5 annual rows.
    let fcf_cagr = if statements.cashflow_annual.len() >= 5 {
        let sorted_rev: Vec<&CashFlowStatement> = {
            let mut v: Vec<&CashFlowStatement> = statements.cashflow_annual.iter().take(5).collect();
            v.sort_by(|a, b| a.date.cmp(&b.date));
            v
        };
        let start = sorted_rev.first().map(|c| c.free_cash_flow).unwrap_or(0.0);
        let end = sorted_rev.last().map(|c| c.free_cash_flow).unwrap_or(0.0);
        if start > 0.0 && end > 0.0 {
            ((end / start).powf(1.0 / 4.0) - 1.0) * 100.0
        } else { 0.0 }
    } else { 0.0 };

    let sustainability_label = if ttm_div <= 0.0 {
        "NO_DIVIDEND".to_string()
    } else if ttm_fcf <= 0.0 || ttm_payout_fcf > 100.0 {
        "UNSUSTAINABLE".to_string()
    } else if ttm_payout_fcf > 75.0 {
        "STRETCHED".to_string()
    } else {
        "SAFE".to_string()
    };

    let note = if periods.is_empty() && ttm_fcf == 0.0 {
        "no cached cash-flow statements — run FA".to_string()
    } else if market_cap <= 0.0 {
        format!("market cap missing — yield pct not computed (last ${:.2})", stock_price)
    } else {
        String::new()
    };

    FcfYieldSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        market_cap,
        ttm_free_cash_flow: ttm_fcf,
        ttm_dividends_paid: ttm_div,
        ttm_fcf_yield_pct: ttm_fcf_yield,
        ttm_dividend_yield_pct: ttm_div_yield,
        ttm_payout_from_fcf_pct: ttm_payout_fcf,
        ttm_payout_from_ni_pct: ttm_payout_ni,
        fcf_cagr_5y_pct: fcf_cagr,
        periods,
        sustainability_label,
        note,
    }
}

// ── ADR-117 Round 10 — SHRT compute (short interest + days-to-cover) ────────

/// Compute a `ShortInterestSnapshot` from the Fundamentals short fields plus
/// daily HP bars. Days-to-cover comes from `short_shares / avg_daily_volume_20d`.
pub fn compute_short_interest_snapshot(
    symbol: &str,
    as_of: &str,
    shares_outstanding: f64,
    shares_float: f64,
    short_percent_of_float: f64,
    short_ratio_reported: f64,
    bars_oldest_first: &[HistoricalPriceRow],
) -> ShortInterestSnapshot {
    let short_shares = if shares_float > 0.0 && short_percent_of_float > 0.0 {
        shares_float * (short_percent_of_float / 100.0)
    } else { 0.0 };

    // 20-day average daily volume from the tail of the bar series.
    let avg_dv_20d = if bars_oldest_first.len() >= 20 {
        let tail = &bars_oldest_first[bars_oldest_first.len() - 20..];
        tail.iter().map(|b| b.volume).sum::<f64>() / 20.0
    } else if !bars_oldest_first.is_empty() {
        bars_oldest_first.iter().map(|b| b.volume).sum::<f64>() / bars_oldest_first.len() as f64
    } else { 0.0 };

    let days_to_cover = if avg_dv_20d > 0.0 && short_shares > 0.0 {
        short_shares / avg_dv_20d
    } else { 0.0 };

    let squeeze_risk_label = if short_shares <= 0.0 || avg_dv_20d <= 0.0 {
        "INSUFFICIENT_DATA".to_string()
    } else if short_percent_of_float >= 30.0 || days_to_cover >= 10.0 {
        "EXTREME".to_string()
    } else if short_percent_of_float >= 20.0 || days_to_cover >= 7.0 {
        "HIGH".to_string()
    } else if short_percent_of_float >= 10.0 || days_to_cover >= 4.0 {
        "ELEVATED".to_string()
    } else {
        "LOW".to_string()
    };

    let note = if short_shares <= 0.0 {
        "no short data in Fundamentals — run EVSCRAPE".to_string()
    } else if bars_oldest_first.is_empty() {
        "no bar volumes — run HP first".to_string()
    } else {
        String::new()
    };

    ShortInterestSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        shares_outstanding,
        shares_float,
        short_shares,
        short_percent_of_float,
        avg_daily_volume_20d: avg_dv_20d,
        days_to_cover,
        short_ratio_reported,
        utilization_proxy_pct: short_percent_of_float,
        squeeze_risk_label,
        note,
    }
}

// ── ADR-118 Godel Parity Round 11 compute fns ──────────────────────────────

/// ALTZ — classic Altman Z-score for public manufacturers.
/// Z = 1.2(WC/TA) + 1.4(RE/TA) + 3.3(EBIT/TA) + 0.6(MVE/TL) + 1.0(Sales/TA)
pub fn compute_altman_z_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
    market_value_equity: f64,
) -> AltmanZSnapshot {
    let bal = statements.balance_annual.first()
        .or_else(|| statements.balance_quarterly.first());
    let inc = statements.income_annual.first()
        .or_else(|| statements.income_quarterly.first());

    let bal = match bal {
        Some(b) => b,
        None => {
            return AltmanZSnapshot {
                symbol: symbol.to_uppercase(),
                as_of: as_of.to_string(),
                zone: "INSUFFICIENT_DATA".to_string(),
                note: "no balance sheet cached — run FA first".to_string(),
                ..Default::default()
            };
        }
    };
    let inc = match inc {
        Some(i) => i,
        None => {
            return AltmanZSnapshot {
                symbol: symbol.to_uppercase(),
                as_of: as_of.to_string(),
                zone: "INSUFFICIENT_DATA".to_string(),
                note: "no income statement cached — run FA first".to_string(),
                ..Default::default()
            };
        }
    };

    let wc = bal.total_current_assets - bal.total_current_liabilities;
    let re = bal.retained_earnings;
    let ebit = inc.operating_income;
    let mve = market_value_equity;
    let sales = inc.revenue;
    let ta = bal.total_assets;
    let tl = bal.total_liabilities;

    if ta <= 0.0 || tl <= 0.0 {
        return AltmanZSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            total_assets: ta,
            total_liabilities: tl,
            zone: "INSUFFICIENT_DATA".to_string(),
            note: "non-positive assets or liabilities".to_string(),
            ..Default::default()
        };
    }

    let a = wc / ta;
    let b = re / ta;
    let c = ebit / ta;
    let d = if tl > 0.0 { mve / tl } else { 0.0 };
    let e = sales / ta;

    let components = vec![
        AltmanComponent { name: "A: WC/TA".into(), ratio: a, coefficient: 1.2, contribution: 1.2 * a, note: "liquidity".into() },
        AltmanComponent { name: "B: RE/TA".into(), ratio: b, coefficient: 1.4, contribution: 1.4 * b, note: "cumulative profitability".into() },
        AltmanComponent { name: "C: EBIT/TA".into(), ratio: c, coefficient: 3.3, contribution: 3.3 * c, note: "operating leverage".into() },
        AltmanComponent { name: "D: MVE/TL".into(), ratio: d, coefficient: 0.6, contribution: 0.6 * d, note: if mve > 0.0 { "solvency" } else { "no market cap" }.into() },
        AltmanComponent { name: "E: Sales/TA".into(), ratio: e, coefficient: 1.0, contribution: 1.0 * e, note: "asset turnover".into() },
    ];

    let z_score: f64 = components.iter().map(|c| c.contribution).sum();
    let zone = if mve <= 0.0 {
        "INSUFFICIENT_DATA".to_string()
    } else if z_score >= 2.99 {
        "SAFE".to_string()
    } else if z_score >= 1.81 {
        "GRAY".to_string()
    } else {
        "DISTRESS".to_string()
    };

    let note = if mve <= 0.0 {
        "no market cap from Fundamentals — D component is zero, zone reports as INSUFFICIENT_DATA".to_string()
    } else {
        String::new()
    };

    AltmanZSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        working_capital: wc,
        retained_earnings: re,
        ebit,
        market_value_equity: mve,
        sales,
        total_assets: ta,
        total_liabilities: tl,
        z_score,
        zone,
        components,
        note,
    }
}

/// PTFS — Piotroski F-score (9-point quality checklist).
/// Requires at least 2 annual periods of FinancialStatements.
pub fn compute_piotroski_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
) -> PiotroskiSnapshot {
    if statements.income_annual.len() < 2
        || statements.balance_annual.len() < 2
        || statements.cashflow_annual.is_empty()
    {
        return PiotroskiSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            strength_label: "INSUFFICIENT_DATA".to_string(),
            note: "need ≥2 annual statements — run FA first".to_string(),
            ..Default::default()
        };
    }

    let inc_cur = &statements.income_annual[0];
    let inc_prev = &statements.income_annual[1];
    let bal_cur = &statements.balance_annual[0];
    let bal_prev = &statements.balance_annual[1];
    let cf_cur = &statements.cashflow_annual[0];

    let ni = inc_cur.net_income;
    let cfo = cf_cur.cash_from_operations;
    let ta_cur = bal_cur.total_assets.max(1.0);
    let ta_prev = bal_prev.total_assets.max(1.0);
    let roa_cur = ni / ta_cur;
    let roa_prev = inc_prev.net_income / ta_prev;
    let accrual_proxy = cfo - ni;

    let ltd_cur = bal_cur.long_term_debt / ta_cur;
    let ltd_prev = bal_prev.long_term_debt / ta_prev;
    let cr_cur = if bal_cur.total_current_liabilities > 0.0 {
        bal_cur.total_current_assets / bal_cur.total_current_liabilities
    } else { 0.0 };
    let cr_prev = if bal_prev.total_current_liabilities > 0.0 {
        bal_prev.total_current_assets / bal_prev.total_current_liabilities
    } else { 0.0 };
    let shares_cur = inc_cur.weighted_shares_out;
    let shares_prev = inc_prev.weighted_shares_out;

    let gm_cur = if inc_cur.revenue > 0.0 { inc_cur.gross_profit / inc_cur.revenue } else { 0.0 };
    let gm_prev = if inc_prev.revenue > 0.0 { inc_prev.gross_profit / inc_prev.revenue } else { 0.0 };
    let at_cur = if ta_cur > 0.0 { inc_cur.revenue / ta_cur } else { 0.0 };
    let at_prev = if ta_prev > 0.0 { inc_prev.revenue / ta_prev } else { 0.0 };

    let mut checks: Vec<PiotroskiCheck> = Vec::new();

    // Profitability (4)
    checks.push(PiotroskiCheck {
        category: "Profitability".into(), name: "Positive Net Income".into(),
        passed: ni > 0.0, value_current: ni, value_prior: 0.0,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Profitability".into(), name: "Positive OCF".into(),
        passed: cfo > 0.0, value_current: cfo, value_prior: 0.0,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Profitability".into(), name: "ROA ↑".into(),
        passed: roa_cur > roa_prev, value_current: roa_cur, value_prior: roa_prev,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Profitability".into(), name: "OCF > NI (accrual)".into(),
        passed: cfo > ni, value_current: cfo, value_prior: ni,
        note: format!("accrual = {:.0}", accrual_proxy),
    });

    // Leverage / Liquidity (3)
    checks.push(PiotroskiCheck {
        category: "Leverage/Liquidity".into(), name: "LT Debt / Assets ↓".into(),
        passed: ltd_cur < ltd_prev, value_current: ltd_cur, value_prior: ltd_prev,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Leverage/Liquidity".into(), name: "Current Ratio ↑".into(),
        passed: cr_cur > cr_prev, value_current: cr_cur, value_prior: cr_prev,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Leverage/Liquidity".into(), name: "No new share issue".into(),
        passed: shares_cur <= shares_prev * 1.005, // 0.5% tolerance for option grants
        value_current: shares_cur, value_prior: shares_prev,
        note: String::new(),
    });

    // Operating Efficiency (2)
    checks.push(PiotroskiCheck {
        category: "Operating Efficiency".into(), name: "Gross Margin ↑".into(),
        passed: gm_cur > gm_prev, value_current: gm_cur, value_prior: gm_prev,
        note: String::new(),
    });
    checks.push(PiotroskiCheck {
        category: "Operating Efficiency".into(), name: "Asset Turnover ↑".into(),
        passed: at_cur > at_prev, value_current: at_cur, value_prior: at_prev,
        note: String::new(),
    });

    let profitability_score: i32 = checks.iter().take(4).filter(|c| c.passed).count() as i32;
    let leverage_score: i32 = checks.iter().skip(4).take(3).filter(|c| c.passed).count() as i32;
    let efficiency_score: i32 = checks.iter().skip(7).take(2).filter(|c| c.passed).count() as i32;
    let f_score = profitability_score + leverage_score + efficiency_score;

    let strength_label = if f_score >= 7 {
        "STRONG".to_string()
    } else if f_score <= 3 {
        "WEAK".to_string()
    } else {
        "MIXED".to_string()
    };

    PiotroskiSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        current_period: inc_cur.date.clone(),
        prior_period: inc_prev.date.clone(),
        f_score,
        strength_label,
        profitability_score,
        leverage_score,
        efficiency_score,
        checks,
        note: String::new(),
    }
}

/// VOLE — OHLC volatility estimators (Parkinson / Garman-Klass / Rogers-Satchell / Yang-Zhang).
/// Needs ≥20 bars with valid OHLC. Uses the tail of the bar series.
pub fn compute_ohlc_vol_snapshot(
    symbol: &str,
    as_of: &str,
    bars_oldest_first: &[HistoricalPriceRow],
    window_days: usize,
) -> OhlcVolSnapshot {
    let needed = window_days.max(20);
    if bars_oldest_first.len() < needed {
        return OhlcVolSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            trading_days: bars_oldest_first.len(),
            note: format!("need ≥{} bars, have {}", needed, bars_oldest_first.len()),
            ..Default::default()
        };
    }

    let tail_start = bars_oldest_first.len() - needed;
    let tail = &bars_oldest_first[tail_start..];
    let n = tail.len();
    let ann = 252.0f64;

    // Valid bars: positive OHLC and high >= low, high >= open, etc.
    let valid: Vec<&HistoricalPriceRow> = tail.iter()
        .filter(|b| b.open > 0.0 && b.high > 0.0 && b.low > 0.0 && b.close > 0.0 && b.high >= b.low)
        .collect();
    if valid.len() < 20 {
        return OhlcVolSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            trading_days: valid.len(),
            note: "fewer than 20 bars with valid OHLC".to_string(),
            ..Default::default()
        };
    }

    // Close-to-close realized vol (baseline).
    let log_ret_cc: Vec<f64> = valid.windows(2)
        .map(|w| (w[1].close / w[0].close).ln())
        .collect();
    let mean_cc: f64 = log_ret_cc.iter().sum::<f64>() / log_ret_cc.len() as f64;
    let var_cc: f64 = log_ret_cc.iter().map(|r| (r - mean_cc).powi(2)).sum::<f64>() / (log_ret_cc.len() - 1).max(1) as f64;
    let ctc_daily = var_cc.sqrt();
    let ctc = ctc_daily * ann.sqrt() * 100.0;

    // Parkinson (range-based).
    // σ² = (1 / (4·ln2·N)) × Σ ln(H/L)²
    let ln2 = 2.0f64.ln();
    let park_sum: f64 = valid.iter()
        .filter(|b| b.low > 0.0)
        .map(|b| (b.high / b.low).ln().powi(2))
        .sum();
    let park_var_daily = park_sum / (4.0 * ln2 * valid.len() as f64);
    let park = park_var_daily.sqrt() * ann.sqrt() * 100.0;

    // Garman-Klass.
    // σ² = (1/N) × Σ [0.5·ln(H/L)² − (2·ln2 − 1)·ln(C/O)²]
    let gk_sum: f64 = valid.iter()
        .filter(|b| b.low > 0.0 && b.open > 0.0)
        .map(|b| {
            let hl = (b.high / b.low).ln();
            let co = (b.close / b.open).ln();
            0.5 * hl * hl - (2.0 * ln2 - 1.0) * co * co
        })
        .sum();
    let gk_var_daily = gk_sum / valid.len() as f64;
    let gk = gk_var_daily.max(0.0).sqrt() * ann.sqrt() * 100.0;

    // Rogers-Satchell (drift-independent).
    // σ² = (1/N) × Σ [ln(H/C)·ln(H/O) + ln(L/C)·ln(L/O)]
    let rs_sum: f64 = valid.iter()
        .filter(|b| b.low > 0.0 && b.open > 0.0 && b.close > 0.0)
        .map(|b| {
            let hc = (b.high / b.close).ln();
            let ho = (b.high / b.open).ln();
            let lc = (b.low / b.close).ln();
            let lo = (b.low / b.open).ln();
            hc * ho + lc * lo
        })
        .sum();
    let rs_var_daily = rs_sum / valid.len() as f64;
    let rs = rs_var_daily.max(0.0).sqrt() * ann.sqrt() * 100.0;

    // Yang-Zhang = overnight_var + k × open-to-close_var + (1-k) × RS_var
    // k = 0.34 / (1.34 + (N+1)/(N-1)). Overnight returns use previous_close → open.
    let overnight_rets: Vec<f64> = valid.windows(2)
        .map(|w| (w[1].open / w[0].close).ln())
        .collect();
    let on_mean: f64 = overnight_rets.iter().sum::<f64>() / overnight_rets.len().max(1) as f64;
    let on_var: f64 = overnight_rets.iter().map(|r| (r - on_mean).powi(2)).sum::<f64>()
        / (overnight_rets.len().saturating_sub(1)).max(1) as f64;
    let oc_rets: Vec<f64> = valid.iter().map(|b| (b.close / b.open).ln()).collect();
    let oc_mean: f64 = oc_rets.iter().sum::<f64>() / oc_rets.len() as f64;
    let oc_var: f64 = oc_rets.iter().map(|r| (r - oc_mean).powi(2)).sum::<f64>()
        / (oc_rets.len() - 1).max(1) as f64;
    let n_f = n as f64;
    let k = 0.34 / (1.34 + (n_f + 1.0) / (n_f - 1.0).max(1.0));
    let yz_var_daily = on_var + k * oc_var + (1.0 - k) * rs_var_daily.max(0.0);
    let yz = yz_var_daily.max(0.0).sqrt() * ann.sqrt() * 100.0;

    let make_row = |name: &str, vol: f64| VolEstimator {
        name: name.to_string(),
        annualized_vol_pct: vol,
        efficiency_vs_close: if ctc > 0.0 { ctc / vol.max(0.0001) } else { 1.0 },
        note: String::new(),
    };

    let estimators = vec![
        make_row("Close-to-Close", ctc),
        make_row("Parkinson", park),
        make_row("Garman-Klass", gk),
        make_row("Rogers-Satchell", rs),
        make_row("Yang-Zhang", yz),
    ];

    let (preferred_label, preferred) = if yz > 0.0 {
        ("Yang-Zhang".to_string(), yz)
    } else if park > 0.0 {
        ("Parkinson".to_string(), park)
    } else {
        ("Close-to-Close".to_string(), ctc)
    };

    OhlcVolSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        trading_days: valid.len(),
        estimators,
        preferred_estimate_pct: preferred,
        preferred_label,
        note: String::new(),
    }
}

/// EPSB — EPS beat streak & surprise analysis over cached earnings-surprise history.
pub fn compute_eps_beat_snapshot(
    symbol: &str,
    as_of: &str,
    reports: &[EarningsSurprise],
) -> EpsBeatSnapshot {
    if reports.is_empty() {
        return EpsBeatSnapshot {
            symbol: symbol.to_uppercase(),
            as_of: as_of.to_string(),
            bias_label: "NEUTRAL".to_string(),
            trend_label: "STABLE".to_string(),
            note: "no EPS surprise history — run EPS first".to_string(),
            ..Default::default()
        };
    }

    // Sort oldest-first by date string (YYYY-MM-DD sorts lexicographically).
    let mut sorted: Vec<EarningsSurprise> = reports.to_vec();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));

    let beats = sorted.iter().filter(|r| r.surprise > 0.0).count();
    let misses = sorted.iter().filter(|r| r.surprise < 0.0).count();
    let inlines = sorted.iter().filter(|r| r.surprise == 0.0).count();
    let total = sorted.len();

    // Beat rate (beats / total).
    let beat_rate_pct = (beats as f64 / total as f64) * 100.0;

    // Current streak: walk from the newest report backwards.
    let mut current_streak: i32 = 0;
    let newest = sorted.last().map(|r| r.surprise).unwrap_or(0.0);
    let direction = if newest > 0.0 { 1i32 } else if newest < 0.0 { -1i32 } else { 0i32 };
    if direction != 0 {
        for r in sorted.iter().rev() {
            if r.surprise > 0.0 && direction == 1 {
                current_streak += 1;
            } else if r.surprise < 0.0 && direction == -1 {
                current_streak -= 1;
            } else {
                break;
            }
        }
    }

    // Longest streaks of each kind.
    let mut longest_beat = 0usize;
    let mut longest_miss = 0usize;
    let mut run_beat = 0usize;
    let mut run_miss = 0usize;
    for r in sorted.iter() {
        if r.surprise > 0.0 {
            run_beat += 1;
            if run_beat > longest_beat { longest_beat = run_beat; }
            run_miss = 0;
        } else if r.surprise < 0.0 {
            run_miss += 1;
            if run_miss > longest_miss { longest_miss = run_miss; }
            run_beat = 0;
        } else {
            run_beat = 0;
            run_miss = 0;
        }
    }

    let avg_surprise_pct = sorted.iter().map(|r| r.surprise_pct).sum::<f64>() / total as f64;
    let mut sorted_pcts: Vec<f64> = sorted.iter().map(|r| r.surprise_pct).collect();
    sorted_pcts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_surprise_pct = if sorted_pcts.len() % 2 == 0 {
        (sorted_pcts[sorted_pcts.len() / 2 - 1] + sorted_pcts[sorted_pcts.len() / 2]) / 2.0
    } else {
        sorted_pcts[sorted_pcts.len() / 2]
    };

    let recent_n = 4.min(sorted.len());
    let recent_slice = &sorted[sorted.len() - recent_n..];
    let recent_avg = recent_slice.iter().map(|r| r.surprise_pct).sum::<f64>() / recent_n as f64;

    let bias_label = if avg_surprise_pct > 2.0 {
        "POSITIVE".to_string()
    } else if avg_surprise_pct < -2.0 {
        "NEGATIVE".to_string()
    } else {
        "NEUTRAL".to_string()
    };

    let trend_label = if recent_avg > avg_surprise_pct + 1.0 {
        "ACCELERATING".to_string()
    } else if recent_avg < avg_surprise_pct - 1.0 {
        "DECELERATING".to_string()
    } else {
        "STABLE".to_string()
    };

    let latest = sorted.last().unwrap();

    EpsBeatSnapshot {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        total_reports: total,
        beats,
        misses,
        inlines,
        beat_rate_pct,
        current_streak,
        longest_beat_streak: longest_beat,
        longest_miss_streak: longest_miss,
        avg_surprise_pct,
        median_surprise_pct,
        recent_avg_surprise_pct: recent_avg,
        bias_label,
        trend_label,
        latest_date: latest.date.clone(),
        latest_surprise_pct: latest.surprise_pct,
        note: String::new(),
    }
}

/// PTD — Price Target Dispersion & Implied Return from cached aggregates.
pub fn compute_price_target_dispersion(
    symbol: &str,
    as_of: &str,
    current_price: f64,
    target: Option<&PriceTarget>,
) -> PriceTargetDispersion {
    let target = match target {
        Some(t) => t,
        None => {
            return PriceTargetDispersion {
                symbol: symbol.to_uppercase(),
                as_of: as_of.to_string(),
                current_price,
                consensus_label: "NO_COVERAGE".to_string(),
                note: "no cached price target — run UPDG / PT first".to_string(),
                ..Default::default()
            };
        }
    };

    let dispersion_pct = if target.target_mean > 0.0 {
        (target.target_high - target.target_low) / target.target_mean * 100.0
    } else { 0.0 };
    let spread_pct = if current_price > 0.0 {
        (target.target_high - target.target_low) / current_price * 100.0
    } else { 0.0 };

    let implied_median = if current_price > 0.0 && target.target_median > 0.0 {
        (target.target_median - current_price) / current_price * 100.0
    } else { 0.0 };
    let implied_mean = if current_price > 0.0 && target.target_mean > 0.0 {
        (target.target_mean - current_price) / current_price * 100.0
    } else { 0.0 };
    let upside_high = if current_price > 0.0 && target.target_high > 0.0 {
        (target.target_high - current_price) / current_price * 100.0
    } else { 0.0 };
    let downside_low = if current_price > 0.0 && target.target_low > 0.0 {
        (target.target_low - current_price) / current_price * 100.0
    } else { 0.0 };

    let consensus_label = if target.num_analysts <= 0 || current_price <= 0.0 {
        "NO_COVERAGE".to_string()
    } else if implied_median >= 10.0 {
        "BULLISH".to_string()
    } else if implied_median <= -5.0 {
        "BEARISH".to_string()
    } else {
        "NEUTRAL".to_string()
    };

    let note = if target.num_analysts <= 0 {
        "target has zero analyst coverage".to_string()
    } else {
        String::new()
    };

    PriceTargetDispersion {
        symbol: symbol.to_uppercase(),
        as_of: as_of.to_string(),
        current_price,
        target_high: target.target_high,
        target_low: target.target_low,
        target_mean: target.target_mean,
        target_median: target.target_median,
        num_analysts: target.num_analysts,
        dispersion_pct,
        spread_pct,
        implied_return_median_pct: implied_median,
        implied_return_mean_pct: implied_mean,
        upside_to_high_pct: upside_high,
        downside_to_low_pct: downside_low,
        consensus_label,
        note,
    }
}

// ── ADR-119 Godel Parity Round 12 compute fns ──────────────────────────────

fn parse_yyyy_mm_dd_to_days(s: &str) -> Option<i64> {
    // Crude julian-ish day number. We don't need calendar correctness — just
    // a monotone integer for sorting & window comparisons against "today".
    let parts: Vec<&str> = s.splitn(3, '-').collect();
    if parts.len() != 3 { return None; }
    let y: i64 = parts[0].parse().ok()?;
    let m: i64 = parts[1].parse().ok()?;
    let d: i64 = parts[2].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) { return None; }
    Some(y * 372 + m * 31 + d)
}

/// MNGR — Insider Activity Bias score computed over a lookback window.
/// Buckets insider trades into buys/sells/other, computes gross/net values,
/// classifies bias from net-value direction and conviction from trade count.
pub fn compute_insider_activity_snapshot(
    symbol: &str,
    as_of: &str,
    trades: &[InsiderTrade],
    window_days: i32,
) -> InsiderActivitySnapshot {
    let sym = symbol.to_uppercase();
    let as_of_days = parse_yyyy_mm_dd_to_days(as_of);
    let cutoff_days = as_of_days.map(|d| d - window_days as i64);

    let in_window: Vec<&InsiderTrade> = trades.iter().filter(|t| {
        match (cutoff_days, parse_yyyy_mm_dd_to_days(&t.transaction_date)) {
            (Some(c), Some(td)) => td >= c,
            _ => true, // if either date unparsable, include it
        }
    }).collect();

    if in_window.is_empty() {
        return InsiderActivitySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            window_days,
            bias_label: "NO_ACTIVITY".to_string(),
            conviction_label: "NONE".to_string(),
            note: "no insider trades in lookback window — run INS first".to_string(),
            ..Default::default()
        };
    }

    let classify = |t: &InsiderTrade| -> &'static str {
        let upper = t.transaction_type.to_uppercase();
        let disp = t.acquisition_disposition.to_uppercase();
        if upper.contains("P-PURCHASE") || upper.starts_with("P ") || upper == "P" || upper.contains("PURCHASE") {
            "buy"
        } else if upper.contains("S-SALE") || upper.starts_with("S ") || upper == "S" || upper.contains("SALE") {
            "sell"
        } else if disp == "A" {
            "buy"
        } else if disp == "D" {
            "sell"
        } else {
            "other"
        }
    };

    let mut buy_count = 0usize;
    let mut sell_count = 0usize;
    let mut other_count = 0usize;
    let mut gross_buy_value = 0.0f64;
    let mut gross_sell_value = 0.0f64;
    let mut net_shares = 0.0f64;
    let mut insiders: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    let mut latest_date = String::new();
    let mut latest_days: i64 = i64::MIN;

    for t in &in_window {
        let v = if t.value_usd.abs() > 0.0 {
            t.value_usd.abs()
        } else {
            (t.shares * t.price).abs()
        };
        match classify(t) {
            "buy" => {
                buy_count += 1;
                gross_buy_value += v;
                net_shares += t.shares.abs();
            }
            "sell" => {
                sell_count += 1;
                gross_sell_value += v;
                net_shares -= t.shares.abs();
            }
            _ => other_count += 1,
        }
        if !t.reporting_name.trim().is_empty() {
            insiders.insert(t.reporting_name.trim().to_lowercase());
        }
        if let Some(td) = parse_yyyy_mm_dd_to_days(&t.transaction_date) {
            if td > latest_days {
                latest_days = td;
                latest_date = t.transaction_date.clone();
            }
        }
    }

    let net_value = gross_buy_value - gross_sell_value;
    let buy_sell_ratio = if sell_count > 0 { buy_count as f64 / sell_count as f64 } else { buy_count as f64 };

    let total_trades = in_window.len();
    let unique = insiders.len();

    let bias = if buy_count == 0 && sell_count == 0 {
        "NO_ACTIVITY"
    } else if net_value > 0.0 && buy_count >= sell_count {
        "BULLISH"
    } else if net_value < 0.0 && sell_count > buy_count {
        "BEARISH"
    } else {
        "NEUTRAL"
    };

    let total_gross = gross_buy_value + gross_sell_value;
    let conviction = if total_gross <= 0.0 || unique == 0 {
        "NONE"
    } else if unique >= 3 && total_gross >= 1_000_000.0 {
        "HIGH"
    } else if unique >= 2 || total_gross >= 250_000.0 {
        "MEDIUM"
    } else {
        "LOW"
    };

    InsiderActivitySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        window_days,
        total_trades,
        buy_count,
        sell_count,
        other_count,
        unique_insiders: unique,
        gross_buy_value_usd: gross_buy_value,
        gross_sell_value_usd: gross_sell_value,
        net_value_usd: net_value,
        buy_sell_ratio,
        net_shares,
        latest_trade_date: latest_date,
        bias_label: bias.to_string(),
        conviction_label: conviction.to_string(),
        note: String::new(),
    }
}

/// DIVG — Dividend Growth Analysis computed over cached DVD rows.
/// Buckets payments by calendar year, computes CAGRs and consistency.
pub fn compute_divg_snapshot(
    symbol: &str,
    as_of: &str,
    dividends: &[DividendRecord],
) -> DivgSnapshot {
    let sym = symbol.to_uppercase();

    if dividends.is_empty() {
        return DivgSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            trend_label: "NO_HISTORY".to_string(),
            note: "no dividend history cached — run DVD first".to_string(),
            ..Default::default()
        };
    }

    // Sort by ex_date ascending
    let mut sorted: Vec<&DividendRecord> = dividends.iter()
        .filter(|d| d.amount > 0.0 && !d.ex_date.is_empty())
        .collect();
    sorted.sort_by(|a, b| a.ex_date.cmp(&b.ex_date));

    if sorted.is_empty() {
        return DivgSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            trend_label: "NO_HISTORY".to_string(),
            note: "dividend rows all zero or missing ex_date".to_string(),
            ..Default::default()
        };
    }

    let first_payment_date = sorted.first().unwrap().ex_date.clone();
    let latest_payment_date = sorted.last().unwrap().ex_date.clone();
    let latest_amount = sorted.last().unwrap().amount;
    let total_payments = sorted.len();

    // Annualized = sum of most recent up-to-4 payments
    let tail_n = sorted.len().min(4);
    let annualized: f64 = sorted.iter().rev().take(tail_n).map(|d| d.amount).sum();

    // Bucket by year
    let mut by_year: std::collections::BTreeMap<i32, (f64, usize)> = std::collections::BTreeMap::new();
    for d in &sorted {
        let year: i32 = match d.ex_date.splitn(2, '-').next().and_then(|y| y.parse().ok()) {
            Some(y) => y,
            None => continue,
        };
        let e = by_year.entry(year).or_insert((0.0, 0));
        e.0 += d.amount;
        e.1 += 1;
    }

    // Determine current year from as_of
    let as_of_year: Option<i32> = as_of.splitn(2, '-').next().and_then(|y| y.parse().ok());

    // Exclude the in-progress current year when it's incomplete (fewer payments than prior year).
    // We still keep prior years as-is. Sort into Vec<(year, amount, count)>.
    let mut years: Vec<(i32, f64, usize)> = by_year.iter().map(|(y, (a, c))| (*y, *a, *c)).collect();
    if let Some(cur) = as_of_year {
        if let Some(last) = years.last() {
            if last.0 == cur {
                let prior_avg_count = if years.len() >= 2 {
                    years[..years.len()-1].iter().rev().take(3).map(|r| r.2).sum::<usize>() as f64 / years.len().min(3) as f64
                } else { 0.0 };
                if (last.2 as f64) < prior_avg_count.max(1.0) * 0.75 {
                    years.pop(); // drop incomplete current year from growth analysis
                }
            }
        }
    }

    let mut annual_rows: Vec<DivgAnnualRow> = Vec::with_capacity(years.len());
    for (i, (y, a, c)) in years.iter().enumerate() {
        let growth = if i == 0 { 0.0 } else {
            let prior = years[i-1].1;
            if prior > 0.0 { (a - prior) / prior * 100.0 } else { 0.0 }
        };
        annual_rows.push(DivgAnnualRow { year: *y, total_amount: *a, payment_count: *c, growth_pct: growth });
    }

    let years_covered = annual_rows.len();
    let cagr = |from: f64, to: f64, n: f64| -> f64 {
        if from <= 0.0 || to <= 0.0 || n <= 0.0 { 0.0 }
        else { ((to / from).powf(1.0 / n) - 1.0) * 100.0 }
    };

    let cagr_1y = if years_covered >= 2 {
        annual_rows.last().unwrap().growth_pct
    } else { 0.0 };
    let cagr_3y = if years_covered >= 4 {
        let n = years_covered;
        cagr(annual_rows[n-4].total_amount, annual_rows[n-1].total_amount, 3.0)
    } else { 0.0 };
    let cagr_5y = if years_covered >= 6 {
        let n = years_covered;
        cagr(annual_rows[n-6].total_amount, annual_rows[n-1].total_amount, 5.0)
    } else { 0.0 };

    // Consecutive growth years counted from the latest backwards
    let mut consecutive = 0usize;
    for row in annual_rows.iter().rev() {
        if row.growth_pct > 0.0 { consecutive += 1; } else { break; }
    }
    // Consecutive counting consumes the latest `consecutive` rows whose growth > 0.
    // The earliest row always has growth_pct = 0.0 so we never count it.

    // Consistency: share of yoy deltas that were non-negative
    let deltas = annual_rows.iter().skip(1).count();
    let non_neg = annual_rows.iter().skip(1).filter(|r| r.growth_pct >= 0.0).count();
    let consistency_pct = if deltas == 0 { 0.0 } else { non_neg as f64 / deltas as f64 * 100.0 };

    let trend_label = if years_covered < 2 {
        "NO_HISTORY"
    } else if cagr_1y >= 3.0 && consistency_pct >= 70.0 {
        "GROWING"
    } else if cagr_1y <= -5.0 {
        "CUTTING"
    } else {
        "STABLE"
    };

    DivgSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        total_payments,
        first_payment_date,
        latest_payment_date,
        latest_amount,
        annualized_dividend: annualized,
        years_covered,
        cagr_1y_pct: cagr_1y,
        cagr_3y_pct: cagr_3y,
        cagr_5y_pct: cagr_5y,
        consecutive_growth_years: consecutive,
        consistency_score_pct: consistency_pct,
        annual_rows,
        trend_label: trend_label.to_string(),
        note: String::new(),
    }
}

/// EARM — Earnings Momentum Trend computed over cached FA + EPS surprises.
pub fn compute_earm_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
    surprises: &[EarningsSurprise],
) -> EarmSnapshot {
    let sym = symbol.to_uppercase();

    let quarters: Vec<&IncomeStatement> = statements.income_quarterly.iter().take(12).collect();

    if quarters.len() < 5 {
        return EarmSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            quarters_used: quarters.len(),
            momentum_label: "INSUFFICIENT_DATA".to_string(),
            note: "need at least 5 quarterly statements — run FA first".to_string(),
            ..Default::default()
        };
    }

    // Assume income_quarterly is newest-first (consistent with other compute fns in this file).
    // quarters[0] = latest, quarters[4] = year ago.
    let mut rows: Vec<EarmQuarterRow> = Vec::with_capacity(quarters.len());
    for (i, q) in quarters.iter().enumerate() {
        let yoy_pct = if i + 4 < quarters.len() {
            let prior = quarters[i + 4].revenue;
            if prior.abs() > 0.0 { (q.revenue - prior) / prior.abs() * 100.0 } else { 0.0 }
        } else { 0.0 };
        let surprise = surprises.iter().find(|s| s.date == q.date);
        rows.push(EarmQuarterRow {
            period: q.date.clone(),
            revenue: q.revenue,
            revenue_yoy_pct: yoy_pct,
            eps_actual: surprise.map(|s| s.eps_actual).unwrap_or(q.eps),
            eps_estimate: surprise.map(|s| s.eps_estimate).unwrap_or(0.0),
            eps_surprise_pct: surprise.map(|s| s.surprise_pct).unwrap_or(0.0),
        });
    }

    // Compute revenue growth averages: latest 4Q vs prior 4Q.
    // Row indices 0..=3 are "recent", 4..=7 are "prior". If we have fewer than 8 rows,
    // use whatever overlap is available for "prior".
    let recent_count = rows.iter().take(4).filter(|r| r.revenue_yoy_pct != 0.0).count();
    let recent_rev_growth: f64 = if recent_count == 0 { 0.0 } else {
        rows.iter().take(4).map(|r| r.revenue_yoy_pct).sum::<f64>() / recent_count as f64
    };
    let prior_slice = if rows.len() >= 8 { &rows[4..8] } else if rows.len() > 4 { &rows[4..] } else { &[] };
    let prior_count = prior_slice.iter().filter(|r| r.revenue_yoy_pct != 0.0).count();
    let prior_rev_growth: f64 = if prior_count == 0 { 0.0 } else {
        prior_slice.iter().map(|r| r.revenue_yoy_pct).sum::<f64>() / prior_count as f64
    };
    let rev_accel = recent_rev_growth - prior_rev_growth;

    // Similar for EPS surprise %. Pull directly from surprises array if FA/surprise alignment is sparse.
    let recent_surprises: Vec<f64> = surprises.iter().take(4).map(|s| s.surprise_pct).collect();
    let prior_surprises: Vec<f64> = surprises.iter().skip(4).take(4).map(|s| s.surprise_pct).collect();
    let recent_eps_surp = if recent_surprises.is_empty() { 0.0 }
        else { recent_surprises.iter().sum::<f64>() / recent_surprises.len() as f64 };
    let prior_eps_surp = if prior_surprises.is_empty() { 0.0 }
        else { prior_surprises.iter().sum::<f64>() / prior_surprises.len() as f64 };
    let eps_accel = recent_eps_surp - prior_eps_surp;

    // Composite 0..100: combine growth level, growth acceleration, surprise level, surprise acceleration.
    // Each component clamped and scaled.
    let clamp = |x: f64, lo: f64, hi: f64| -> f64 { x.max(lo).min(hi) };
    let g1 = (clamp(recent_rev_growth, -30.0, 30.0) + 30.0) / 60.0 * 100.0;
    let g2 = (clamp(rev_accel,          -15.0, 15.0) + 15.0) / 30.0 * 100.0;
    let g3 = (clamp(recent_eps_surp,    -30.0, 30.0) + 30.0) / 60.0 * 100.0;
    let g4 = (clamp(eps_accel,          -15.0, 15.0) + 15.0) / 30.0 * 100.0;
    let composite = (g1 * 0.35 + g2 * 0.25 + g3 * 0.25 + g4 * 0.15).max(0.0).min(100.0);

    let momentum = if composite >= 65.0 && (rev_accel > 0.0 || eps_accel > 0.0) {
        "ACCELERATING"
    } else if composite <= 35.0 && (rev_accel < 0.0 || eps_accel < 0.0) {
        "DECELERATING"
    } else {
        "STABLE"
    };

    EarmSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        quarters_used: quarters.len(),
        recent_revenue_growth_pct: recent_rev_growth,
        prior_revenue_growth_pct: prior_rev_growth,
        revenue_acceleration_pct: rev_accel,
        recent_eps_surprise_pct: recent_eps_surp,
        prior_eps_surprise_pct: prior_eps_surp,
        eps_surprise_acceleration_pct: eps_accel,
        composite_score: composite,
        momentum_label: momentum.to_string(),
        quarters: rows,
        note: String::new(),
    }
}

/// SECTR — Sector Rotation Strength for a symbol, using the latest INDU snapshot.
pub fn compute_sector_rotation_snapshot(
    symbol: &str,
    as_of: &str,
    symbol_sector: &str,
    sectors: &[SectorPerformance],
) -> SectorRotationSnapshot {
    let sym = symbol.to_uppercase();

    if sectors.is_empty() {
        return SectorRotationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            symbol_sector: symbol_sector.to_string(),
            strength_label: "NO_DATA".to_string(),
            note: "no sector performance cached — run INDU first".to_string(),
            ..Default::default()
        };
    }

    let mut ranked: Vec<&SectorPerformance> = sectors.iter().collect();
    ranked.sort_by(|a, b| b.change_pct.partial_cmp(&a.change_pct).unwrap_or(std::cmp::Ordering::Equal));

    let sectors_total = ranked.len() as i32;
    let avg_change = ranked.iter().map(|s| s.change_pct).sum::<f64>() / ranked.len() as f64;

    let mut sorted_pcts: Vec<f64> = ranked.iter().map(|s| s.change_pct).collect();
    sorted_pcts.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_change = if sorted_pcts.is_empty() { 0.0 }
        else if sorted_pcts.len() % 2 == 1 { sorted_pcts[sorted_pcts.len() / 2] }
        else { (sorted_pcts[sorted_pcts.len() / 2 - 1] + sorted_pcts[sorted_pcts.len() / 2]) / 2.0 };

    let breadth = ranked.iter().filter(|s| s.change_pct > 0.0).count() as f64 / ranked.len() as f64 * 100.0;

    let strongest = ranked.first().unwrap();
    let weakest = ranked.last().unwrap();

    // Locate symbol's sector. Fuzzy-match: exact, case-insensitive, contains.
    let target = symbol_sector.trim();
    let target_lower = target.to_lowercase();
    let (symbol_rank, symbol_change) = if target.is_empty() {
        (0i32, 0.0f64)
    } else {
        let mut rank = 0i32;
        let mut change = 0.0f64;
        for (i, s) in ranked.iter().enumerate() {
            let a = s.sector.to_lowercase();
            if a == target_lower || a.contains(&target_lower) || target_lower.contains(&a) {
                rank = (i + 1) as i32;
                change = s.change_pct;
                break;
            }
        }
        (rank, change)
    };

    let rel_strength = symbol_change - avg_change;

    let strength = if symbol_rank == 0 {
        "NO_DATA"
    } else if symbol_rank <= (sectors_total / 3).max(1) && rel_strength > 0.0 {
        "LEADER"
    } else if symbol_rank > sectors_total - (sectors_total / 3).max(1) && rel_strength < 0.0 {
        "LAGGARD"
    } else {
        "NEUTRAL"
    };

    let note = if symbol_rank == 0 && !target.is_empty() {
        format!("symbol sector '{}' not found in cached INDU snapshot", target)
    } else {
        String::new()
    };

    SectorRotationSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        symbol_sector: symbol_sector.to_string(),
        symbol_sector_change_pct: symbol_change,
        sector_rank: symbol_rank,
        sectors_total,
        avg_sector_change_pct: avg_change,
        median_sector_change_pct: median_change,
        relative_strength_pct: rel_strength,
        breadth_pct: breadth,
        strongest_sector: strongest.sector.clone(),
        strongest_sector_pct: strongest.change_pct,
        weakest_sector: weakest.sector.clone(),
        weakest_sector_pct: weakest.change_pct,
        strength_label: strength.to_string(),
        note,
    }
}

/// UPDM — Upgrade/Downgrade Momentum snapshot for a symbol.
pub fn compute_updm_snapshot(
    symbol: &str,
    as_of: &str,
    actions: &[RatingChange],
) -> UpdmSnapshot {
    let sym = symbol.to_uppercase();
    let as_of_days = parse_yyyy_mm_dd_to_days(as_of);

    if actions.is_empty() || as_of_days.is_none() {
        return UpdmSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bias_label: "NO_COVERAGE".to_string(),
            trend_label: "STABLE".to_string(),
            note: "no rating change history cached — run UPDG first".to_string(),
            ..Default::default()
        };
    }
    let as_of_days = as_of_days.unwrap();

    let (mut up30, mut dn30, mut up90, mut dn90, mut up180, mut dn180) = (0,0,0,0,0,0);
    let mut init90 = 0;
    let mut maint90 = 0;
    let mut total = 0;
    let mut latest_days: i64 = i64::MIN;
    let mut latest: Option<&RatingChange> = None;

    for a in actions {
        total += 1;
        let ad = match parse_yyyy_mm_dd_to_days(&a.date) { Some(d) => d, None => continue };
        let delta = as_of_days - ad;
        if delta < 0 { continue; }
        let act = a.action.to_lowercase();
        let is_up = act.contains("upgrade");
        let is_dn = act.contains("downgrade");
        let is_init = act.contains("init");
        let is_maint = act.contains("maintain") || act.contains("reiterat");

        if delta <= 30 {
            if is_up { up30 += 1; }
            if is_dn { dn30 += 1; }
        }
        if delta <= 90 {
            if is_up { up90 += 1; }
            if is_dn { dn90 += 1; }
            if is_init { init90 += 1; }
            if is_maint { maint90 += 1; }
        }
        if delta <= 180 {
            if is_up { up180 += 1; }
            if is_dn { dn180 += 1; }
        }

        if ad > latest_days {
            latest_days = ad;
            latest = Some(a);
        }
    }

    let net_30 = up30 as i32 - dn30 as i32;
    let net_90 = up90 as i32 - dn90 as i32;
    let net_180 = up180 as i32 - dn180 as i32;

    let bias = if up90 == 0 && dn90 == 0 && init90 == 0 && maint90 == 0 {
        "NO_COVERAGE"
    } else if net_90 > 0 {
        "BULLISH"
    } else if net_90 < 0 {
        "BEARISH"
    } else {
        "NEUTRAL"
    };

    let trend = if net_30 > 0 && net_30 as i64 * 3 >= net_90 as i64 {
        "IMPROVING"
    } else if net_30 < 0 && net_30 as i64 * 3 <= net_90 as i64 {
        "DETERIORATING"
    } else {
        "STABLE"
    };

    let (latest_date, latest_action, latest_firm, latest_grade) = latest.map(|l| (
        l.date.clone(), l.action.clone(), l.firm.clone(), l.to_grade.clone(),
    )).unwrap_or_default();

    UpdmSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        total_actions: total,
        upgrades_30d: up30,
        downgrades_30d: dn30,
        upgrades_90d: up90,
        downgrades_90d: dn90,
        upgrades_180d: up180,
        downgrades_180d: dn180,
        initiations_90d: init90,
        maintains_90d: maint90,
        net_30d: net_30,
        net_90d: net_90,
        net_180d: net_180,
        latest_date,
        latest_action,
        latest_firm,
        latest_to_grade: latest_grade,
        bias_label: bias.to_string(),
        trend_label: trend.to_string(),
        note: String::new(),
    }
}

// ── ADR-120 Godel Parity Round 13 compute fns ──────────────────────────────

/// Pick the daily close closest to (and not after) `target_offset_back` bars
/// from the most recent bar. `bars` is newest-first. Returns None if the
/// offset is out of range.
fn pick_close_offset(bars_newest_first: &[HistoricalPriceRow], offset: usize) -> Option<f64> {
    if offset >= bars_newest_first.len() { return None; }
    let c = bars_newest_first[offset].close;
    if c > 0.0 { Some(c) } else { None }
}

/// MOM — 12-1 month momentum snapshot.
pub fn compute_momentum_snapshot(
    symbol: &str,
    as_of: &str,
    bars_newest_first: &[HistoricalPriceRow],
) -> MomentumSnapshot {
    let sym = symbol.to_uppercase();
    let n = bars_newest_first.len();

    if n < 252 {
        return MomentumSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: n as i32,
            regime_label: "INSUFFICIENT_DATA".to_string(),
            trend_label: "STABLE".to_string(),
            note: format!("need ≥252 bars; have {n}"),
            ..Default::default()
        };
    }

    let current = bars_newest_first[0].close;
    let c_1m = pick_close_offset(bars_newest_first, 21).unwrap_or(current);
    let c_3m = pick_close_offset(bars_newest_first, 63).unwrap_or(current);
    let c_6m = pick_close_offset(bars_newest_first, 126).unwrap_or(current);
    let c_12m = pick_close_offset(bars_newest_first, 252).unwrap_or(current);

    let pct = |from: f64, to: f64| -> f64 {
        if from > 0.0 { (to - from) / from * 100.0 } else { 0.0 }
    };
    let return_1m = pct(c_1m, current);
    let return_3m = pct(c_3m, current);
    let return_6m = pct(c_6m, current);
    let return_12m = pct(c_12m, current);
    // 12-1 = return from 12m ago to 1m ago (skipping the most recent month)
    let return_12_1 = pct(c_12m, c_1m);

    // Annualised daily return stdev over the last 252 bars.
    let mut log_rets: Vec<f64> = Vec::with_capacity(251);
    for i in 0..251 {
        let c_new = bars_newest_first[i].close;
        let c_old = bars_newest_first[i + 1].close;
        if c_new > 0.0 && c_old > 0.0 {
            log_rets.push((c_new / c_old).ln());
        }
    }
    let mean: f64 = if log_rets.is_empty() { 0.0 } else { log_rets.iter().sum::<f64>() / log_rets.len() as f64 };
    let var: f64 = if log_rets.len() > 1 {
        log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / (log_rets.len() - 1) as f64
    } else { 0.0 };
    let daily_stdev = var.sqrt();
    let vol_ann_pct = daily_stdev * (252f64).sqrt() * 100.0;

    let vol_adj_score = if vol_ann_pct > 0.0 { return_12_1 / vol_ann_pct } else { 0.0 };

    let composite = (50.0 + vol_adj_score * 20.0 + return_6m * 0.3).clamp(0.0, 100.0);
    let regime = if composite >= 75.0 { "STRONG" }
                 else if composite >= 40.0 { "NEUTRAL" }
                 else if composite >= 20.0 { "WEAK" }
                 else { "CRASH" };
    let trend = if return_1m > return_3m / 3.0 && return_3m > return_6m / 2.0 { "ACCELERATING" }
                else if return_1m < return_3m / 3.0 && return_3m < return_6m / 2.0 { "DECELERATING" }
                else { "STABLE" };

    MomentumSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: n as i32,
        return_1m_pct: return_1m,
        return_3m_pct: return_3m,
        return_6m_pct: return_6m,
        return_12m_pct: return_12m,
        return_12_1_pct: return_12_1,
        vol_annualized_pct: vol_ann_pct,
        vol_adjusted_score: vol_adj_score,
        composite_score: composite,
        regime_label: regime.to_string(),
        trend_label: trend.to_string(),
        note: String::new(),
    }
}

/// LIQ — Liquidity profile snapshot.
pub fn compute_liquidity_snapshot(
    symbol: &str,
    as_of: &str,
    bars_newest_first: &[HistoricalPriceRow],
    shares_outstanding: f64,
    window_days: i32,
) -> LiquiditySnapshot {
    let sym = symbol.to_uppercase();
    let w = window_days.max(20) as usize;

    if bars_newest_first.len() < 20 {
        return LiquiditySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            window_days: w as i32,
            shares_outstanding,
            liquidity_tier: "INSUFFICIENT_DATA".to_string(),
            note: format!("need ≥20 bars; have {}", bars_newest_first.len()),
            ..Default::default()
        };
    }

    let slice_len = bars_newest_first.len().min(w);
    let slice = &bars_newest_first[..slice_len];

    let mut share_vols: Vec<f64> = Vec::with_capacity(slice_len);
    let mut dollar_vols: Vec<f64> = Vec::with_capacity(slice_len);
    let mut true_range_pcts: Vec<f64> = Vec::with_capacity(slice_len);
    let mut amihud_terms: Vec<f64> = Vec::new();
    let mut high_low_betas: Vec<f64> = Vec::new();

    for (i, b) in slice.iter().enumerate() {
        if b.volume > 0.0 {
            share_vols.push(b.volume);
            let dv = b.volume * b.close;
            dollar_vols.push(dv);
            if b.high > 0.0 && b.low > 0.0 && b.high >= b.low {
                let hl = b.high - b.low;
                if b.close > 0.0 {
                    true_range_pcts.push(hl / b.close * 100.0);
                }
                // Corwin-Schultz beta term — ln²(H/L)
                if b.high > 0.0 && b.low > 0.0 {
                    let ln_hl = (b.high / b.low).ln();
                    high_low_betas.push(ln_hl * ln_hl);
                }
            }
            // Amihud: |daily return| / dollar volume
            if i + 1 < slice.len() {
                let prev = slice[i + 1].close;
                if prev > 0.0 && dv > 0.0 {
                    let r = (b.close - prev) / prev;
                    amihud_terms.push(r.abs() / dv);
                }
            }
        }
    }

    let avg_share = if share_vols.is_empty() { 0.0 } else { share_vols.iter().sum::<f64>() / share_vols.len() as f64 };
    let avg_dollar = if dollar_vols.is_empty() { 0.0 } else { dollar_vols.iter().sum::<f64>() / dollar_vols.len() as f64 };
    let median = |mut v: Vec<f64>| -> f64 {
        if v.is_empty() { return 0.0; }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let mid = v.len() / 2;
        if v.len() % 2 == 0 { (v[mid - 1] + v[mid]) / 2.0 } else { v[mid] }
    };
    let med_share = median(share_vols.clone());
    let med_dollar = median(dollar_vols.clone());

    let turnover_pct = if shares_outstanding > 0.0 { avg_share / shares_outstanding * 100.0 } else { 0.0 };
    let amihud = if amihud_terms.is_empty() {
        0.0
    } else {
        amihud_terms.iter().sum::<f64>() / amihud_terms.len() as f64 * 1.0e6
    };
    let atr_pct = if true_range_pcts.is_empty() { 0.0 } else { true_range_pcts.iter().sum::<f64>() / true_range_pcts.len() as f64 };
    // Corwin-Schultz simplified: spread% ≈ 2 · (exp(α) − 1) / (1 + exp(α))
    // where α = (√(2β) − √β) / (3 − 2√2) and β is the average of ln²(H/L).
    let spread_proxy_pct = if high_low_betas.is_empty() {
        0.0
    } else {
        let beta = high_low_betas.iter().sum::<f64>() / high_low_betas.len() as f64;
        let denom = 3.0 - 2.0 * (2f64).sqrt();
        if denom > 0.0 && beta >= 0.0 {
            let alpha = ((2.0 * beta).sqrt() - beta.sqrt()) / denom;
            let ea = alpha.exp();
            if ea + 1.0 > 0.0 {
                (2.0 * (ea - 1.0) / (ea + 1.0)) * 100.0
            } else { 0.0 }
        } else { 0.0 }
    };

    let tier = if avg_dollar >= 5.0e8 {
        "DEEP"
    } else if avg_dollar >= 5.0e7 {
        "LIQUID"
    } else if avg_dollar >= 5.0e6 {
        "MODERATE"
    } else if avg_dollar >= 5.0e5 {
        "THIN"
    } else {
        "ILLIQUID"
    };

    LiquiditySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        window_days: w as i32,
        avg_daily_share_volume: avg_share,
        median_daily_share_volume: med_share,
        avg_daily_dollar_volume: avg_dollar,
        median_daily_dollar_volume: med_dollar,
        shares_outstanding,
        daily_turnover_pct: turnover_pct,
        amihud_illiquidity: amihud,
        avg_true_range_pct: atr_pct,
        spread_proxy_pct: spread_proxy_pct.max(0.0),
        liquidity_tier: tier.to_string(),
        note: String::new(),
    }
}

/// BREAK — Breakout proximity snapshot.
pub fn compute_breakout_snapshot(
    symbol: &str,
    as_of: &str,
    bars_newest_first: &[HistoricalPriceRow],
) -> BreakoutSnapshot {
    let sym = symbol.to_uppercase();
    let n = bars_newest_first.len();

    if n < 20 {
        return BreakoutSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            breakout_label: "INSUFFICIENT_DATA".to_string(),
            setup_label: "NEUTRAL".to_string(),
            note: format!("need ≥20 bars; have {n}"),
            ..Default::default()
        };
    }

    let current = bars_newest_first[0].close;

    let range_high_low = |slice: &[HistoricalPriceRow]| -> (f64, f64) {
        let mut hi = f64::MIN;
        let mut lo = f64::MAX;
        for b in slice {
            if b.high > 0.0 && b.high > hi { hi = b.high; }
            if b.low > 0.0 && b.low < lo { lo = b.low; }
        }
        if hi == f64::MIN { hi = 0.0; }
        if lo == f64::MAX { lo = 0.0; }
        (hi, lo)
    };

    let (h20, l20) = range_high_low(&bars_newest_first[..20.min(n)]);
    let (h60, l60) = range_high_low(&bars_newest_first[..60.min(n)]);
    let (h52, l52) = range_high_low(&bars_newest_first[..252.min(n)]);

    let pct_from = |target: f64, from: f64| -> f64 {
        if from > 0.0 { (target - from) / from * 100.0 } else { 0.0 }
    };

    let dist_52w_high = pct_from(current, h52);
    let dist_52w_low = pct_from(current, l52);
    let dist_20d_high = pct_from(current, h20);
    let dist_60d_high = pct_from(current, h60);

    let pos_in_range = |cur: f64, hi: f64, lo: f64| -> f64 {
        let width = hi - lo;
        if width > 0.0 { (cur - lo) / width * 100.0 } else { 50.0 }
    };
    let pos_52w = pos_in_range(current, h52, l52);
    let pos_20d = pos_in_range(current, h20, l20);

    let cons_pct = {
        let mean_close = {
            let mut s = 0.0;
            let mut k = 0;
            for b in &bars_newest_first[..20.min(n)] {
                if b.close > 0.0 { s += b.close; k += 1; }
            }
            if k > 0 { s / k as f64 } else { current }
        };
        if mean_close > 0.0 { (h20 - l20) / mean_close * 100.0 } else { 0.0 }
    };

    let breakout = if pos_52w >= 99.0 && current >= h52 {
        "NEW_HIGH"
    } else if pos_52w >= 85.0 {
        "NEAR_HIGH"
    } else if pos_52w >= 15.0 {
        "MID_RANGE"
    } else if pos_52w >= 1.0 {
        "NEAR_LOW"
    } else {
        "NEW_LOW"
    };

    let setup = if cons_pct < 8.0 && pos_20d >= 70.0 {
        "BREAKOUT_IMMINENT"
    } else if cons_pct < 6.0 {
        "CONSOLIDATING"
    } else if dist_60d_high.abs() < 3.0 && pos_52w >= 60.0 {
        "TRENDING_UP"
    } else if pos_52w <= 35.0 && dist_52w_low < 10.0 {
        "TRENDING_DOWN"
    } else {
        "NEUTRAL"
    };

    BreakoutSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        current_price: current,
        high_20d: h20,
        low_20d: l20,
        high_60d: h60,
        low_60d: l60,
        high_52w: h52,
        low_52w: l52,
        dist_from_52w_high_pct: dist_52w_high,
        dist_from_52w_low_pct: dist_52w_low,
        dist_from_20d_high_pct: dist_20d_high,
        dist_from_60d_high_pct: dist_60d_high,
        position_in_52w_range_pct: pos_52w,
        position_in_20d_range_pct: pos_20d,
        consolidation_pct: cons_pct,
        breakout_label: breakout.to_string(),
        setup_label: setup.to_string(),
        note: String::new(),
    }
}

/// CCRL — Cash conversion cycle snapshot.
pub fn compute_cash_cycle_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
) -> CashCycleSnapshot {
    let sym = symbol.to_uppercase();

    let (income, balance, basis) = if !statements.income_annual.is_empty() && !statements.balance_annual.is_empty() {
        (&statements.income_annual, &statements.balance_annual, "annual")
    } else if !statements.income_quarterly.is_empty() && !statements.balance_quarterly.is_empty() {
        (&statements.income_quarterly, &statements.balance_quarterly, "quarterly")
    } else {
        return CashCycleSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            efficiency_label: "INSUFFICIENT_DATA".to_string(),
            trend_label: "STABLE".to_string(),
            note: "need cached FA annual or quarterly statements".to_string(),
            ..Default::default()
        };
    };

    let days_factor: f64 = if basis == "annual" { 365.0 } else { 91.25 };

    let compute_row = |inc: &IncomeStatement, bal: &BalanceSheet| -> Option<CashCycleRow> {
        if inc.revenue <= 0.0 || inc.cost_of_revenue <= 0.0 { return None; }
        let dso = bal.net_receivables / inc.revenue * days_factor;
        let dio = bal.inventory / inc.cost_of_revenue * days_factor;
        let dpo = bal.accounts_payable / inc.cost_of_revenue * days_factor;
        let ccc = dso + dio - dpo;
        Some(CashCycleRow {
            period: inc.date.clone(),
            dso_days: dso,
            dio_days: dio,
            dpo_days: dpo,
            ccc_days: ccc,
        })
    };

    let pair_count = income.len().min(balance.len());
    let mut rows: Vec<CashCycleRow> = Vec::with_capacity(pair_count);
    for i in 0..pair_count {
        if let Some(r) = compute_row(&income[i], &balance[i]) {
            rows.push(r);
        }
    }

    if rows.is_empty() {
        return CashCycleSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            efficiency_label: "INSUFFICIENT_DATA".to_string(),
            trend_label: "STABLE".to_string(),
            note: "revenue or COGS missing / zero in cached statements".to_string(),
            ..Default::default()
        };
    }

    let latest = &rows[0];
    let prior = rows.get(1);
    let prior_ccc = prior.map(|p| p.ccc_days).unwrap_or(latest.ccc_days);
    let change = latest.ccc_days - prior_ccc;

    let avg_window = rows.iter().take(3).map(|r| r.ccc_days).collect::<Vec<_>>();
    let avg_3y = avg_window.iter().sum::<f64>() / avg_window.len() as f64;

    let efficiency = if latest.ccc_days < 30.0 {
        "EFFICIENT"
    } else if latest.ccc_days < 90.0 {
        "NEUTRAL"
    } else {
        "INEFFICIENT"
    };

    let trend = if change <= -5.0 {
        "IMPROVING"
    } else if change >= 5.0 {
        "DETERIORATING"
    } else {
        "STABLE"
    };

    CashCycleSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        latest_period: latest.period.clone(),
        dso_days: latest.dso_days,
        dio_days: latest.dio_days,
        dpo_days: latest.dpo_days,
        ccc_days: latest.ccc_days,
        prior_ccc_days: prior_ccc,
        ccc_change_days: change,
        ccc_3y_avg_days: avg_3y,
        periods_used: rows.len(),
        efficiency_label: efficiency.to_string(),
        trend_label: trend.to_string(),
        periods: rows,
        note: String::new(),
    }
}

/// CREDIT — Unified credit score fusing ALTZ + PTFS + LEV + ACRL snapshots.
pub fn compute_credit_snapshot(
    symbol: &str,
    as_of: &str,
    altman: Option<&AltmanZSnapshot>,
    piotroski: Option<&PiotroskiSnapshot>,
    leverage: Option<&LeverageSnapshot>,
    accruals: Option<&AccrualsSnapshot>,
) -> CreditSnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<CreditComponent> = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;

    let mut altman_z = 0.0;
    let mut altman_zone = String::new();
    let mut piotroski_score = 0;
    let mut piotroski_label = String::new();
    let mut leverage_summary = String::new();
    let mut leverage_score = 0.0;
    let mut accruals_trend = String::new();
    let mut accruals_ttm = 0.0;
    let mut inputs_available = 0usize;

    // ALTZ — weight 35. Map Z via piecewise linear: DISTRESS<1.81→0..30, GRAY→30..70, SAFE≥2.99→70..100.
    if let Some(a) = altman {
        if a.zone != "INSUFFICIENT_DATA" && !a.zone.is_empty() {
            altman_z = a.z_score;
            altman_zone = a.zone.clone();
            let z = a.z_score;
            let score = if z >= 2.99 {
                let extra = (z - 2.99).min(3.0);
                (70.0 + extra / 3.0 * 30.0).min(100.0)
            } else if z >= 1.81 {
                let t = (z - 1.81) / (2.99 - 1.81);
                30.0 + t * 40.0
            } else if z > 0.0 {
                (z / 1.81 * 30.0).clamp(0.0, 30.0)
            } else {
                0.0
            };
            let w = 35.0;
            components.push(CreditComponent {
                name: "Altman Z".to_string(),
                value: format!("Z {:.2} ({})", z, a.zone),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // PTFS — weight 25. Map F score linearly 0..9 → 0..100.
    if let Some(p) = piotroski {
        if p.strength_label != "INSUFFICIENT_DATA" && !p.strength_label.is_empty() {
            piotroski_score = p.f_score;
            piotroski_label = p.strength_label.clone();
            let score = (p.f_score as f64 / 9.0 * 100.0).clamp(0.0, 100.0);
            let w = 25.0;
            components.push(CreditComponent {
                name: "Piotroski F".to_string(),
                value: format!("{}/9 ({})", p.f_score, p.strength_label),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // LEV — weight 25. Map solvency_summary label to a score.
    if let Some(lv) = leverage {
        if !lv.solvency_summary.is_empty() {
            leverage_summary = lv.solvency_summary.clone();
            let score = match lv.solvency_summary.as_str() {
                "HEALTHY" => 85.0,
                "MODERATE" | "NEUTRAL" => 60.0,
                "ELEVATED" => 40.0,
                "STRETCHED" | "DISTRESSED" => 15.0,
                _ => 50.0,
            };
            leverage_score = score;
            let w = 25.0;
            components.push(CreditComponent {
                name: "Leverage".to_string(),
                value: lv.solvency_summary.clone(),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // ACRL — weight 15. Map trend_label and ttm cash conversion to a score.
    if let Some(ac) = accruals {
        if !ac.trend_label.is_empty() {
            accruals_trend = ac.trend_label.clone();
            accruals_ttm = ac.ttm_cash_conversion_pct;
            let mut score: f64 = match ac.trend_label.as_str() {
                "IMPROVING" => 80.0,
                "STABLE" => 60.0,
                "MIXED" => 50.0,
                "DETERIORATING" => 30.0,
                _ => 50.0,
            };
            // Cash conversion >100% is a positive lean; <50% drags.
            if ac.ttm_cash_conversion_pct >= 100.0 {
                score = (score + 10.0).min(100.0);
            } else if ac.ttm_cash_conversion_pct < 50.0 && ac.ttm_cash_conversion_pct != 0.0 {
                score = (score - 10.0).max(0.0);
            }
            let w = 15.0;
            components.push(CreditComponent {
                name: "Accruals".to_string(),
                value: format!("{} ({:.0}% cash conv)", ac.trend_label, ac.ttm_cash_conversion_pct),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return CreditSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            letter_grade: "INSUFFICIENT_DATA".to_string(),
            credit_label: "INSUFFICIENT_DATA".to_string(),
            inputs_available: 0,
            note: "need at least one of ALTZ / PTFS / LEV / ACRL cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let letter = if composite >= 90.0 { "AAA" }
                 else if composite >= 80.0 { "AA" }
                 else if composite >= 70.0 { "A" }
                 else if composite >= 60.0 { "BBB" }
                 else if composite >= 50.0 { "BB" }
                 else if composite >= 35.0 { "B" }
                 else { "CCC" };
    let label = if composite >= 70.0 { "INVESTMENT_GRADE" }
                else if composite >= 55.0 { "BORDERLINE" }
                else if composite >= 35.0 { "SPECULATIVE" }
                else { "DISTRESSED" };

    CreditSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        altman_z,
        altman_zone,
        piotroski_score,
        piotroski_label,
        leverage_summary,
        leverage_score,
        accruals_trend,
        accruals_ttm_cash_conversion_pct: accruals_ttm,
        composite_score: composite,
        letter_grade: letter.to_string(),
        credit_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

// ── ADR-121 Round 14 compute fns ───────────────────────────────────────────

/// GROWM — Growth-at-Reasonable-Price fusion of MOM + EARM + DIVG.
pub fn compute_growm_snapshot(
    symbol: &str,
    as_of: &str,
    momentum: Option<&MomentumSnapshot>,
    earm: Option<&EarmSnapshot>,
    divg: Option<&DivgSnapshot>,
) -> GrowmSnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<GarpComponent> = Vec::new();
    let mut weighted_sum = 0.0;
    let mut total_weight = 0.0;
    let mut inputs_available = 0usize;

    let mut momentum_score = 0.0;
    let mut momentum_regime = String::new();
    let mut earm_score = 0.0;
    let mut earm_label = String::new();
    let mut divg_cagr = 0.0;
    let mut divg_trend = String::new();

    // MOM — weight 40. Composite is already 0..100.
    if let Some(m) = momentum {
        if m.regime_label != "INSUFFICIENT_DATA" && !m.regime_label.is_empty() {
            momentum_score = m.composite_score;
            momentum_regime = m.regime_label.clone();
            let w = 40.0;
            components.push(GarpComponent {
                name: "Momentum 12-1".to_string(),
                value: format!("{} ({:.1})", m.regime_label, m.composite_score),
                score: momentum_score,
                weight: w,
                contribution: momentum_score * w / 100.0,
            });
            weighted_sum += momentum_score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // EARM — weight 40. Composite is already 0..100.
    if let Some(e) = earm {
        if e.momentum_label != "INSUFFICIENT_DATA" && !e.momentum_label.is_empty() {
            earm_score = e.composite_score;
            earm_label = e.momentum_label.clone();
            let w = 40.0;
            components.push(GarpComponent {
                name: "Earnings Momentum".to_string(),
                value: format!("{} ({:.1})", e.momentum_label, e.composite_score),
                score: earm_score,
                weight: w,
                contribution: earm_score * w / 100.0,
            });
            weighted_sum += earm_score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // DIVG — weight 20. Map 3Y CAGR and trend to a score.
    if let Some(d) = divg {
        if d.trend_label != "NO_HISTORY" && !d.trend_label.is_empty() {
            divg_cagr = d.cagr_3y_pct;
            divg_trend = d.trend_label.clone();
            let mut score: f64 = match d.trend_label.as_str() {
                "GROWING" => 70.0,
                "STABLE" => 55.0,
                "CUTTING" => 25.0,
                _ => 50.0,
            };
            // Boost / penalty from the 3Y CAGR itself.
            if d.cagr_3y_pct >= 10.0 { score = (score + 15.0).min(100.0); }
            else if d.cagr_3y_pct >= 5.0 { score = (score + 7.0).min(100.0); }
            else if d.cagr_3y_pct < -5.0 { score = (score - 15.0).max(0.0); }
            let w = 20.0;
            components.push(GarpComponent {
                name: "Dividend Growth".to_string(),
                value: format!("{} (3Y {:+.1}%)", d.trend_label, d.cagr_3y_pct),
                score,
                weight: w,
                contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return GrowmSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            garp_label: "NO_DATA".to_string(),
            inputs_available: 0,
            note: "need at least one of MOM / EARM / DIVG cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    // GARP: balance momentum + earnings growth. Pure GROWTH = high MOM but weak EARM. VALUE = dividend-led. SPECULATIVE = only MOM.
    let mom_has = !momentum_regime.is_empty();
    let earm_has = !earm_label.is_empty();
    let divg_has = !divg_trend.is_empty();
    let label = if composite >= 70.0 && mom_has && earm_has {
        "GARP"
    } else if composite >= 65.0 && mom_has {
        "GROWTH"
    } else if composite >= 55.0 && divg_has && !earm_has {
        "VALUE"
    } else if composite >= 50.0 {
        if mom_has && !earm_has { "SPECULATIVE" } else { "GARP" }
    } else if composite >= 35.0 {
        "VALUE"
    } else {
        "SPECULATIVE"
    };

    GrowmSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        momentum_score,
        momentum_regime,
        earnings_momentum_score: earm_score,
        earnings_label: earm_label,
        dividend_cagr_3y_pct: divg_cagr,
        dividend_trend: divg_trend,
        composite_score: composite,
        garp_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

/// FLOW — Smart-money flow snapshot (insider + institutional).
pub fn compute_flow_snapshot(
    symbol: &str,
    as_of: &str,
    insider_trades: &[InsiderTrade],
    holders: &[InstitutionalHolder],
    window_days: i32,
) -> FlowSnapshot {
    let sym = symbol.to_uppercase();
    let w = window_days.max(7);

    let as_of_days_opt = parse_yyyy_mm_dd_to_days(as_of);
    let cutoff_opt = as_of_days_opt.map(|a| a - (w as i64 * 31 / 30).max(1));

    let mut buy_value = 0.0f64;
    let mut sell_value = 0.0f64;
    let mut trade_count = 0usize;
    let mut names: std::collections::HashSet<String> = std::collections::HashSet::new();

    for t in insider_trades {
        if t.transaction_date.is_empty() { continue; }
        let d = parse_yyyy_mm_dd_to_days(&t.transaction_date);
        if let (Some(cut), Some(dd)) = (cutoff_opt, d) {
            if dd < cut { continue; }
        }
        trade_count += 1;
        if !t.reporting_name.is_empty() { names.insert(t.reporting_name.clone()); }
        let kind = t.transaction_type.to_ascii_lowercase();
        if kind.contains('p') && kind.contains("purchase") {
            buy_value += t.value_usd.abs();
        } else if kind.contains('s') && kind.contains("sale") {
            sell_value += t.value_usd.abs();
        } else if t.acquisition_disposition.eq_ignore_ascii_case("a") {
            buy_value += t.value_usd.abs();
        } else if t.acquisition_disposition.eq_ignore_ascii_case("d") {
            sell_value += t.value_usd.abs();
        }
    }
    let insider_net = buy_value - sell_value;

    // Institutional flows: use HDS `change` column (delta vs prior 13F).
    let mut positive_delta = 0.0f64;
    let mut negative_delta = 0.0f64;
    let mut buyers = 0usize;
    let mut sellers = 0usize;
    let tracked = holders.len();
    for h in holders {
        if h.change > 0.0 {
            positive_delta += h.change;
            buyers += 1;
        } else if h.change < 0.0 {
            negative_delta += h.change.abs();
            sellers += 1;
        }
    }
    let net_share_delta = positive_delta - negative_delta;
    let net_ratio = if tracked > 0 {
        (buyers as f64 - sellers as f64) / tracked as f64
    } else {
        0.0
    };

    // Insider score: buy_value vs total activity.
    let gross_insider = buy_value + sell_value;
    let insider_score: f64 = if gross_insider <= 0.0 {
        50.0
    } else {
        let ratio = insider_net / gross_insider; // -1..1
        (50.0 + ratio * 50.0).clamp(0.0, 100.0)
    };

    // Institutional score: net_ratio -1..1 → 0..100.
    let institutional_score: f64 = if tracked == 0 {
        50.0
    } else {
        (50.0 + net_ratio * 50.0).clamp(0.0, 100.0)
    };

    let any_insider = trade_count > 0;
    let any_institutional = tracked > 0;

    let composite: f64 = if !any_insider && !any_institutional {
        return FlowSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            window_days: w,
            flow_label: "NO_DATA".to_string(),
            note: "need cached INS or HDS rows".to_string(),
            ..Default::default()
        };
    } else if any_insider && any_institutional {
        // weight insider 60, institutional 40 — insiders are more load-bearing signal
        (insider_score * 0.6 + institutional_score * 0.4).clamp(0.0, 100.0)
    } else if any_insider {
        insider_score
    } else {
        institutional_score
    };

    let label = if composite >= 80.0 { "STRONG_BUY" }
                else if composite >= 60.0 { "BUY" }
                else if composite >= 40.0 { "NEUTRAL" }
                else if composite >= 20.0 { "SELL" }
                else { "STRONG_SELL" };

    FlowSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        window_days: w,
        insider_buy_value_usd: buy_value,
        insider_sell_value_usd: sell_value,
        insider_net_value_usd: insider_net,
        insider_trade_count: trade_count,
        unique_insiders: names.len(),
        institutional_share_delta: net_share_delta,
        institutional_buyers: buyers,
        institutional_sellers: sellers,
        institutional_holders_tracked: tracked,
        institutional_net_ratio: net_ratio,
        insider_score,
        institutional_score,
        composite_score: composite,
        flow_label: label.to_string(),
        note: String::new(),
    }
}

/// REGIME — regime classifier fusing VOLE + TECH + HRA.
pub fn compute_regime_snapshot(
    symbol: &str,
    as_of: &str,
    vole: Option<&OhlcVolSnapshot>,
    tech: Option<&TechnicalSnapshot>,
    hra: Option<&HraSnapshot>,
) -> RegimeSnapshot {
    let sym = symbol.to_uppercase();
    let mut inputs_available = 0usize;

    let mut realized_vol_pct = 0.0;
    let mut vol_source = String::new();
    let mut adx_value = 0.0;
    let mut trend_summary = String::new();
    let mut sharpe = 0.0;
    let mut return_1y = 0.0;

    if let Some(v) = vole {
        if v.preferred_estimate_pct > 0.0 {
            realized_vol_pct = v.preferred_estimate_pct;
            vol_source = v.preferred_label.clone();
            inputs_available += 1;
        }
    }

    if let Some(t) = tech {
        trend_summary = t.trend_summary.clone();
        for ind in &t.indicators {
            if ind.name.to_ascii_uppercase().starts_with("ADX") {
                adx_value = ind.value;
                break;
            }
        }
        if !trend_summary.is_empty() || adx_value > 0.0 {
            inputs_available += 1;
        }
    }

    if let Some(h) = hra {
        sharpe = h.sharpe_ratio;
        for w in &h.windows {
            if w.label.eq_ignore_ascii_case("1Y") || w.label == "1y" {
                return_1y = w.return_pct;
                break;
            }
        }
        if h.volatility_annual_pct > 0.0 || !h.windows.is_empty() {
            inputs_available += 1;
        }
    }

    if inputs_available == 0 {
        return RegimeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            regime_label: "INSUFFICIENT_DATA".to_string(),
            inputs_available: 0,
            note: "need at least one of VOLE / TECH / HRA cached".to_string(),
            ..Default::default()
        };
    }

    // Trend strength from ADX (25+ = strong trend).
    let trend_strength: f64 = if adx_value <= 0.0 {
        50.0
    } else if adx_value >= 40.0 { 100.0 }
    else if adx_value >= 25.0 { 60.0 + (adx_value - 25.0) / 15.0 * 40.0 }
    else if adx_value >= 15.0 { 30.0 + (adx_value - 15.0) / 10.0 * 30.0 }
    else { (adx_value / 15.0 * 30.0).max(0.0) };

    // Volatility score: low vol = high score.
    let vol_score: f64 = if realized_vol_pct <= 0.0 {
        50.0
    } else if realized_vol_pct < 15.0 { 90.0 }
    else if realized_vol_pct < 25.0 { 70.0 }
    else if realized_vol_pct < 40.0 { 50.0 }
    else if realized_vol_pct < 60.0 { 30.0 }
    else { 10.0 };

    // Return score from 1Y: +20% → 80, -20% → 20.
    let return_score: f64 = (50.0 + return_1y * 1.5).clamp(0.0, 100.0);

    let composite = ((trend_strength + vol_score + return_score) / 3.0).clamp(0.0, 100.0);

    // Regime classification.
    let regime = if realized_vol_pct >= 40.0 {
        "VOLATILE"
    } else if adx_value >= 25.0 && return_score >= 55.0 {
        "TRENDING"
    } else if adx_value >= 20.0 {
        "TRENDING"
    } else if realized_vol_pct > 0.0 && realized_vol_pct < 20.0 && adx_value < 18.0 {
        "QUIET"
    } else if adx_value < 20.0 {
        "MEAN_REVERTING"
    } else {
        "MEAN_REVERTING"
    };

    RegimeSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        realized_vol_pct,
        vol_source,
        adx_value,
        trend_summary,
        sharpe_ratio: sharpe,
        return_1y_pct: return_1y,
        trend_strength_score: trend_strength,
        volatility_score: vol_score,
        return_score,
        composite_score: composite,
        regime_label: regime.to_string(),
        inputs_available,
        note: String::new(),
    }
}

/// RELVOL — Relative volume snapshot over 5d/20d/60d windows.
pub fn compute_relvol_snapshot(
    symbol: &str,
    as_of: &str,
    bars_newest_first: &[HistoricalPriceRow],
) -> RelVolSnapshot {
    let sym = symbol.to_uppercase();
    let n = bars_newest_first.len();

    if n < 20 {
        return RelVolSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: n,
            activity_label: "INSUFFICIENT_DATA".to_string(),
            direction_label: "NEUTRAL".to_string(),
            note: format!("need ≥20 bars; have {n}"),
            ..Default::default()
        };
    }

    let current = bars_newest_first[0].volume;
    let avg = |slice: &[HistoricalPriceRow]| -> f64 {
        if slice.is_empty() { return 0.0; }
        let mut s = 0.0; let mut k = 0;
        for b in slice { if b.volume > 0.0 { s += b.volume; k += 1; } }
        if k > 0 { s / k as f64 } else { 0.0 }
    };
    // Averages exclude the current bar to prevent the current bar from skewing the baseline.
    let avg_5 = avg(&bars_newest_first[1..(1 + 5).min(n)]);
    let avg_20 = avg(&bars_newest_first[1..(1 + 20).min(n)]);
    let avg_60 = avg(&bars_newest_first[1..(1 + 60).min(n)]);

    let rel = |num: f64, den: f64| -> f64 { if den > 0.0 { num / den } else { 0.0 } };
    let r5 = rel(current, avg_5);
    let r20 = rel(current, avg_20);
    let r60 = rel(current, avg_60);

    let vol_trend = if avg_20 > 0.0 { (avg_5 / avg_20 - 1.0) * 100.0 } else { 0.0 };

    // Percentile rank of current vs last 60 bars (excluding itself).
    let sample_end = (1 + 60).min(n);
    let sample: Vec<f64> = bars_newest_first[1..sample_end].iter().map(|b| b.volume).collect();
    let percentile = if sample.is_empty() {
        50.0
    } else {
        let count_below = sample.iter().filter(|v| **v < current).count();
        count_below as f64 / sample.len() as f64 * 100.0
    };

    let activity = if r20 >= 3.0 { "EXTREME" }
                   else if r20 >= 2.0 { "HIGH" }
                   else if r20 >= 1.5 { "ELEVATED" }
                   else if r20 >= 0.5 { "NORMAL" }
                   else { "LOW" };

    let direction = if n >= 2 {
        let prior_close = bars_newest_first[1].close;
        let now_close = bars_newest_first[0].close;
        if prior_close > 0.0 && now_close > prior_close * 1.005 { "BULLISH" }
        else if prior_close > 0.0 && now_close < prior_close * 0.995 { "BEARISH" }
        else { "NEUTRAL" }
    } else { "NEUTRAL" };

    RelVolSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        current_volume: current,
        avg_volume_5d: avg_5,
        avg_volume_20d: avg_20,
        avg_volume_60d: avg_60,
        rel_volume_5d: r5,
        rel_volume_20d: r20,
        rel_volume_60d: r60,
        volume_trend_5d_pct: vol_trend,
        volume_percentile_60d: percentile,
        activity_label: activity.to_string(),
        direction_label: direction.to_string(),
        bars_used: n,
        note: String::new(),
    }
}

/// MARGINS — Margin trajectory snapshot (gross / operating / net).
pub fn compute_margins_snapshot(
    symbol: &str,
    as_of: &str,
    statements: &FinancialStatements,
) -> MarginsSnapshot {
    let sym = symbol.to_uppercase();

    let (income, basis) = if !statements.income_annual.is_empty() {
        (&statements.income_annual, "annual")
    } else if !statements.income_quarterly.is_empty() {
        (&statements.income_quarterly, "quarterly")
    } else {
        return MarginsSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            gross_trend_label: "INSUFFICIENT_DATA".to_string(),
            operating_trend_label: "INSUFFICIENT_DATA".to_string(),
            net_trend_label: "INSUFFICIENT_DATA".to_string(),
            overall_trend_label: "INSUFFICIENT_DATA".to_string(),
            quality_label: "INSUFFICIENT_DATA".to_string(),
            note: "need cached FA annual or quarterly income statements".to_string(),
            ..Default::default()
        };
    };

    let mut rows: Vec<MarginRow> = Vec::new();
    for inc in income.iter() {
        if inc.revenue <= 0.0 { continue; }
        let g = if inc.gross_profit != 0.0 { inc.gross_profit / inc.revenue * 100.0 } else { 0.0 };
        let o = if inc.operating_income != 0.0 { inc.operating_income / inc.revenue * 100.0 } else { 0.0 };
        let n_m = inc.net_income / inc.revenue * 100.0;
        rows.push(MarginRow {
            period: inc.date.clone(),
            gross_margin_pct: g,
            operating_margin_pct: o,
            net_margin_pct: n_m,
        });
    }

    if rows.is_empty() {
        return MarginsSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            basis: basis.to_string(),
            gross_trend_label: "INSUFFICIENT_DATA".to_string(),
            operating_trend_label: "INSUFFICIENT_DATA".to_string(),
            net_trend_label: "INSUFFICIENT_DATA".to_string(),
            overall_trend_label: "INSUFFICIENT_DATA".to_string(),
            quality_label: "INSUFFICIENT_DATA".to_string(),
            note: "no periods with positive revenue in cached statements".to_string(),
            ..Default::default()
        };
    }

    let latest = &rows[0];
    let prior = rows.get(1).cloned().unwrap_or_else(|| latest.clone());
    let g_chg = latest.gross_margin_pct - prior.gross_margin_pct;
    let o_chg = latest.operating_margin_pct - prior.operating_margin_pct;
    let n_chg = latest.net_margin_pct - prior.net_margin_pct;

    let avg_g = rows.iter().map(|r| r.gross_margin_pct).sum::<f64>() / rows.len() as f64;
    let avg_o = rows.iter().map(|r| r.operating_margin_pct).sum::<f64>() / rows.len() as f64;
    let avg_n = rows.iter().map(|r| r.net_margin_pct).sum::<f64>() / rows.len() as f64;

    let label_trend = |chg: f64| -> &'static str {
        if chg >= 1.0 { "EXPANDING" }
        else if chg <= -1.0 { "CONTRACTING" }
        else { "STABLE" }
    };
    let gross_trend = label_trend(g_chg);
    let op_trend = label_trend(o_chg);
    let net_trend = label_trend(n_chg);

    // Overall — majority rule across the three.
    let mut exp_n = 0; let mut con_n = 0;
    for t in [gross_trend, op_trend, net_trend] {
        if t == "EXPANDING" { exp_n += 1; }
        else if t == "CONTRACTING" { con_n += 1; }
    }
    let overall = if exp_n >= 2 { "EXPANDING" }
                  else if con_n >= 2 { "CONTRACTING" }
                  else { "STABLE" };

    let quality = if latest.operating_margin_pct >= 20.0 { "HIGH" }
                  else if latest.operating_margin_pct >= 8.0 { "MEDIUM" }
                  else { "LOW" };

    MarginsSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        basis: basis.to_string(),
        latest_period: latest.period.clone(),
        latest_gross_margin_pct: latest.gross_margin_pct,
        latest_operating_margin_pct: latest.operating_margin_pct,
        latest_net_margin_pct: latest.net_margin_pct,
        prior_gross_margin_pct: prior.gross_margin_pct,
        prior_operating_margin_pct: prior.operating_margin_pct,
        prior_net_margin_pct: prior.net_margin_pct,
        gross_margin_change_pct: g_chg,
        operating_margin_change_pct: o_chg,
        net_margin_change_pct: n_chg,
        avg_gross_margin_pct: avg_g,
        avg_operating_margin_pct: avg_o,
        avg_net_margin_pct: avg_n,
        periods_used: rows.len(),
        gross_trend_label: gross_trend.to_string(),
        operating_trend_label: op_trend.to_string(),
        net_trend_label: net_trend.to_string(),
        overall_trend_label: overall.to_string(),
        quality_label: quality.to_string(),
        periods: rows,
        note: String::new(),
    }
}

// ── ADR-122 Round 15 compute fns ───────────────────────────────────────────

fn median_f64(values: &[f64]) -> f64 {
    if values.is_empty() { return 0.0; }
    let mut v: Vec<f64> = values.iter().copied().filter(|x| x.is_finite()).collect();
    v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    if v.is_empty() { return 0.0; }
    let mid = v.len() / 2;
    if v.len() % 2 == 0 { (v[mid - 1] + v[mid]) / 2.0 } else { v[mid] }
}

/// Score a "lower is better" multiple vs a peer median.
/// ratio ≤ median × 0.5 → 100; ratio ≥ median × 2.0 → 0; linear in between.
fn score_multiple_lower_better(value: f64, median: f64) -> f64 {
    if !value.is_finite() || value <= 0.0 || !median.is_finite() || median <= 0.0 {
        return 0.0;
    }
    let ratio = value / median;
    if ratio <= 0.5 { 100.0 }
    else if ratio >= 2.0 { 0.0 }
    else { (100.0 * (2.0 - ratio) / 1.5).clamp(0.0, 100.0) }
}

/// Score a "higher is better" yield vs a peer median.
/// yield ≥ median × 1.5 → 100; yield ≤ median × 0.5 → 0; linear in between.
fn score_yield_higher_better(value: f64, median: f64) -> f64 {
    if !value.is_finite() || !median.is_finite() || median <= 0.0 {
        return 0.0;
    }
    let ratio = value / median;
    if ratio >= 1.5 { 100.0 }
    else if ratio <= 0.5 { 0.0 }
    else { (100.0 * (ratio - 0.5) / 1.0).clamp(0.0, 100.0) }
}

/// VAL — Value-factor composite vs sector peers.
pub fn compute_val_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    fund: Option<&crate::core::fundamentals::Fundamentals>,
    peer_fundamentals: &[crate::core::fundamentals::Fundamentals],
    fcfy: Option<&FcfYieldSnapshot>,
    peer_fcf_yields: &[f64],
) -> ValueSnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<FactorComponent> = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    let mut inputs_available = 0usize;

    let f = match fund {
        Some(v) => v,
        None => {
            return ValueSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                value_label: "NO_DATA".to_string(),
                note: "no Fundamentals row cached for this symbol".to_string(),
                ..Default::default()
            };
        }
    };

    let peers_considered = peer_fundamentals.len();

    // Collect peer medians for each metric — only non-missing, positive values.
    let peer_pe: Vec<f64> = peer_fundamentals.iter()
        .filter_map(|p| p.pe_ratio).filter(|v| *v > 0.0 && v.is_finite()).collect();
    let peer_fpe: Vec<f64> = peer_fundamentals.iter()
        .filter_map(|p| p.forward_pe).filter(|v| *v > 0.0 && v.is_finite()).collect();
    let peer_pb: Vec<f64> = peer_fundamentals.iter()
        .filter_map(|p| p.price_to_book).filter(|v| *v > 0.0 && v.is_finite()).collect();
    let peer_ps: Vec<f64> = peer_fundamentals.iter()
        .filter_map(|p| p.price_to_sales).filter(|v| *v > 0.0 && v.is_finite()).collect();
    let peer_evebitda: Vec<f64> = peer_fundamentals.iter()
        .filter_map(|p| p.ev_to_ebitda).filter(|v| *v > 0.0 && v.is_finite()).collect();

    let pe_median = median_f64(&peer_pe);
    let fpe_median = median_f64(&peer_fpe);
    let pb_median = median_f64(&peer_pb);
    let ps_median = median_f64(&peer_ps);
    let evebitda_median = median_f64(&peer_evebitda);
    let fcfy_median = median_f64(peer_fcf_yields);

    let pe = f.pe_ratio.unwrap_or(0.0);
    let fpe = f.forward_pe.unwrap_or(0.0);
    let pb = f.price_to_book.unwrap_or(0.0);
    let ps = f.price_to_sales.unwrap_or(0.0);
    let evebitda = f.ev_to_ebitda.unwrap_or(0.0);
    let fcfy_val = fcfy.map(|s| s.ttm_fcf_yield_pct).unwrap_or(0.0);

    // P/E — weight 25
    if pe > 0.0 && pe_median > 0.0 {
        let score = score_multiple_lower_better(pe, pe_median);
        let w = 25.0;
        components.push(FactorComponent {
            name: "P/E".to_string(),
            value: format!("{:.2} vs median {:.2}", pe, pe_median),
            score, weight: w, contribution: score * w / 100.0,
        });
        weighted_sum += score * w;
        total_weight += w;
        inputs_available += 1;
    }

    // Forward P/E — weight 15
    if fpe > 0.0 && fpe_median > 0.0 {
        let score = score_multiple_lower_better(fpe, fpe_median);
        let w = 15.0;
        components.push(FactorComponent {
            name: "Forward P/E".to_string(),
            value: format!("{:.2} vs median {:.2}", fpe, fpe_median),
            score, weight: w, contribution: score * w / 100.0,
        });
        weighted_sum += score * w;
        total_weight += w;
        inputs_available += 1;
    }

    // P/B — weight 15
    if pb > 0.0 && pb_median > 0.0 {
        let score = score_multiple_lower_better(pb, pb_median);
        let w = 15.0;
        components.push(FactorComponent {
            name: "P/B".to_string(),
            value: format!("{:.2} vs median {:.2}", pb, pb_median),
            score, weight: w, contribution: score * w / 100.0,
        });
        weighted_sum += score * w;
        total_weight += w;
        inputs_available += 1;
    }

    // P/S — weight 15
    if ps > 0.0 && ps_median > 0.0 {
        let score = score_multiple_lower_better(ps, ps_median);
        let w = 15.0;
        components.push(FactorComponent {
            name: "P/S".to_string(),
            value: format!("{:.2} vs median {:.2}", ps, ps_median),
            score, weight: w, contribution: score * w / 100.0,
        });
        weighted_sum += score * w;
        total_weight += w;
        inputs_available += 1;
    }

    // EV/EBITDA — weight 20
    if evebitda > 0.0 && evebitda_median > 0.0 {
        let score = score_multiple_lower_better(evebitda, evebitda_median);
        let w = 20.0;
        components.push(FactorComponent {
            name: "EV/EBITDA".to_string(),
            value: format!("{:.2} vs median {:.2}", evebitda, evebitda_median),
            score, weight: w, contribution: score * w / 100.0,
        });
        weighted_sum += score * w;
        total_weight += w;
        inputs_available += 1;
    }

    // FCF Yield — weight 10
    if fcfy_val.is_finite() && fcfy_median > 0.0 {
        let score = score_yield_higher_better(fcfy_val, fcfy_median);
        let w = 10.0;
        components.push(FactorComponent {
            name: "FCF Yield".to_string(),
            value: format!("{:.2}% vs median {:.2}%", fcfy_val, fcfy_median),
            score, weight: w, contribution: score * w / 100.0,
        });
        weighted_sum += score * w;
        total_weight += w;
        inputs_available += 1;
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return ValueSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            peers_considered,
            value_label: "NO_DATA".to_string(),
            note: "need at least one valuation metric vs a non-empty sector peer median".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let label = if composite >= 80.0 { "DEEP_VALUE" }
                else if composite >= 65.0 { "VALUE" }
                else if composite >= 45.0 { "FAIR" }
                else if composite >= 30.0 { "EXPENSIVE" }
                else { "PREMIUM" };

    ValueSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        peers_considered,
        pe_ratio: pe,
        pe_sector_median: pe_median,
        forward_pe: fpe,
        forward_pe_sector_median: fpe_median,
        price_to_book: pb,
        price_to_book_sector_median: pb_median,
        price_to_sales: ps,
        price_to_sales_sector_median: ps_median,
        ev_to_ebitda: evebitda,
        ev_to_ebitda_sector_median: evebitda_median,
        fcf_yield_pct: fcfy_val,
        fcf_yield_sector_median_pct: fcfy_median,
        composite_score: composite,
        value_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

/// QUAL — Quality-factor composite fusing PTFS + MARGINS + ACRL + LEV.
pub fn compute_qual_snapshot(
    symbol: &str,
    as_of: &str,
    piotroski: Option<&PiotroskiSnapshot>,
    margins: Option<&MarginsSnapshot>,
    accruals: Option<&AccrualsSnapshot>,
    leverage: Option<&LeverageSnapshot>,
) -> QualitySnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<FactorComponent> = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    let mut inputs_available = 0usize;

    let mut piotroski_score = 0;
    let mut piotroski_label = String::new();
    let mut operating_margin_pct = 0.0;
    let mut margin_trend_label = String::new();
    let mut cash_conversion_pct = 0.0;
    let mut accruals_trend_label = String::new();
    let mut leverage_summary = String::new();
    let mut debt_to_ebitda = 0.0;

    // PTFS — weight 30. Map F score linearly 0..9 → 0..100.
    if let Some(p) = piotroski {
        if p.strength_label != "INSUFFICIENT_DATA" && !p.strength_label.is_empty() {
            piotroski_score = p.f_score;
            piotroski_label = p.strength_label.clone();
            let score = (p.f_score as f64 / 9.0 * 100.0).clamp(0.0, 100.0);
            let w = 30.0;
            components.push(FactorComponent {
                name: "Piotroski F".to_string(),
                value: format!("{}/9 ({})", p.f_score, p.strength_label),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // MARGINS — weight 25. Fuse quality_label bucket + trend bonus.
    if let Some(m) = margins {
        if m.quality_label != "INSUFFICIENT_DATA" && !m.quality_label.is_empty() {
            operating_margin_pct = m.latest_operating_margin_pct;
            margin_trend_label = m.overall_trend_label.clone();
            let mut score: f64 = match m.quality_label.as_str() {
                "HIGH" => 85.0,
                "MEDIUM" => 60.0,
                "LOW" => 30.0,
                _ => 50.0,
            };
            match m.overall_trend_label.as_str() {
                "EXPANDING" => score = (score + 10.0).min(100.0),
                "CONTRACTING" => score = (score - 10.0).max(0.0),
                _ => {}
            }
            let w = 25.0;
            components.push(FactorComponent {
                name: "Margins".to_string(),
                value: format!("{} op {:.1}% ({})", m.quality_label, m.latest_operating_margin_pct, m.overall_trend_label),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // ACRL — weight 25. Fuse trend_label + ttm cash conversion bonus.
    if let Some(ac) = accruals {
        if !ac.trend_label.is_empty() {
            accruals_trend_label = ac.trend_label.clone();
            cash_conversion_pct = ac.ttm_cash_conversion_pct;
            let mut score: f64 = match ac.trend_label.as_str() {
                "IMPROVING" => 80.0,
                "STABLE" => 60.0,
                "MIXED" => 50.0,
                "DETERIORATING" => 30.0,
                _ => 50.0,
            };
            if ac.ttm_cash_conversion_pct >= 100.0 {
                score = (score + 10.0).min(100.0);
            } else if ac.ttm_cash_conversion_pct < 50.0 && ac.ttm_cash_conversion_pct != 0.0 {
                score = (score - 10.0).max(0.0);
            }
            let w = 25.0;
            components.push(FactorComponent {
                name: "Accruals".to_string(),
                value: format!("{} ({:.0}% cash conv)", ac.trend_label, ac.ttm_cash_conversion_pct),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // LEV — weight 20. Map solvency_summary label to a score + debt/ebitda.
    if let Some(lv) = leverage {
        if !lv.solvency_summary.is_empty() {
            leverage_summary = lv.solvency_summary.clone();
            debt_to_ebitda = if lv.ebitda_ttm > 0.0 { lv.total_debt / lv.ebitda_ttm } else { 0.0 };
            let score = match lv.solvency_summary.as_str() {
                "HEALTHY" => 85.0,
                "MODERATE" | "NEUTRAL" => 60.0,
                "ELEVATED" => 40.0,
                "STRETCHED" | "DISTRESSED" => 15.0,
                _ => 50.0,
            };
            let w = 20.0;
            components.push(FactorComponent {
                name: "Leverage".to_string(),
                value: format!("{} (D/EBITDA {:.2})", lv.solvency_summary, debt_to_ebitda),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return QualitySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            quality_label: "NO_DATA".to_string(),
            note: "need at least one of PTFS / MARGINS / ACRL / LEV cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let label = if composite >= 80.0 { "HIGH_QUALITY" }
                else if composite >= 65.0 { "QUALITY" }
                else if composite >= 45.0 { "AVERAGE" }
                else if composite >= 30.0 { "POOR" }
                else { "WEAK" };

    QualitySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        piotroski_score,
        piotroski_label,
        operating_margin_pct,
        margin_trend_label,
        cash_conversion_pct,
        accruals_trend_label,
        leverage_summary,
        debt_to_ebitda,
        composite_score: composite,
        quality_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

/// RISK — Risk-factor composite fusing VOLE + BETA + LIQ + SHRT + ALTZ.
/// Higher composite_score = RISKIER.
pub fn compute_risk_snapshot(
    symbol: &str,
    as_of: &str,
    vole: Option<&OhlcVolSnapshot>,
    beta: Option<&BetaSnapshot>,
    liquidity: Option<&LiquiditySnapshot>,
    short_interest: Option<&ShortInterestSnapshot>,
    altman: Option<&AltmanZSnapshot>,
) -> RiskSnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<FactorComponent> = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    let mut inputs_available = 0usize;

    let mut realized_vol_pct = 0.0;
    let mut beta_1y = 0.0;
    let mut liquidity_tier = String::new();
    let mut short_percent_of_float = 0.0;
    let mut days_to_cover = 0.0;
    let mut altman_z = 0.0;
    let mut altman_zone = String::new();
    let mut distressed = false;

    // VOLE — weight 25. Higher vol → higher risk score.
    // 10% vol = 0, 30% = 50, 60% = 100 (linear piecewise).
    if let Some(v) = vole {
        if v.preferred_estimate_pct > 0.0 {
            realized_vol_pct = v.preferred_estimate_pct;
            let score = if v.preferred_estimate_pct <= 10.0 { 0.0 }
                        else if v.preferred_estimate_pct <= 30.0 {
                            (v.preferred_estimate_pct - 10.0) / 20.0 * 50.0
                        } else if v.preferred_estimate_pct <= 60.0 {
                            50.0 + (v.preferred_estimate_pct - 30.0) / 30.0 * 50.0
                        } else { 100.0 };
            let w = 25.0;
            components.push(FactorComponent {
                name: "Realized Vol".to_string(),
                value: format!("{:.1}% ({})", v.preferred_estimate_pct, v.preferred_label),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // BETA — weight 20. |β - 1| contributes to risk; high |β| far from 1 = high risk.
    if let Some(b) = beta {
        if let Some(one_y) = b.windows.iter().find(|w| w.window_label == "1Y") {
            if one_y.n_observations > 0 {
                beta_1y = one_y.beta;
                let dist = (one_y.beta - 1.0).abs();
                let score = (dist / 1.0 * 60.0).min(100.0);   // |β-1|=1 → 60; |β-1|>=1.67 → 100
                let w = 20.0;
                components.push(FactorComponent {
                    name: "Beta 1Y".to_string(),
                    value: format!("β {:.2}", one_y.beta),
                    score, weight: w, contribution: score * w / 100.0,
                });
                weighted_sum += score * w;
                total_weight += w;
                inputs_available += 1;
            }
        }
    }

    // LIQ — weight 15. Thin liquidity = high risk.
    if let Some(l) = liquidity {
        if l.liquidity_tier != "INSUFFICIENT_DATA" && !l.liquidity_tier.is_empty() {
            liquidity_tier = l.liquidity_tier.clone();
            let score = match l.liquidity_tier.as_str() {
                "DEEP" => 5.0,
                "LIQUID" => 20.0,
                "MODERATE" => 45.0,
                "THIN" => 75.0,
                "ILLIQUID" => 95.0,
                _ => 50.0,
            };
            let w = 15.0;
            components.push(FactorComponent {
                name: "Liquidity".to_string(),
                value: l.liquidity_tier.clone(),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // SHRT — weight 15. High short % of float + days to cover = high squeeze / sentiment risk.
    if let Some(s) = short_interest {
        if s.squeeze_risk_label != "INSUFFICIENT_DATA" && !s.squeeze_risk_label.is_empty() {
            short_percent_of_float = s.short_percent_of_float;
            days_to_cover = s.days_to_cover;
            let score = match s.squeeze_risk_label.as_str() {
                "LOW" => 20.0,
                "ELEVATED" => 55.0,
                "HIGH" => 80.0,
                "EXTREME" => 100.0,
                _ => 40.0,
            };
            let w = 15.0;
            components.push(FactorComponent {
                name: "Short Interest".to_string(),
                value: format!("{:.1}% float, {:.1} DTC ({})", s.short_percent_of_float, s.days_to_cover, s.squeeze_risk_label),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // ALTZ — weight 25. DISTRESS zone = highest risk.
    if let Some(a) = altman {
        if a.zone != "INSUFFICIENT_DATA" && !a.zone.is_empty() {
            altman_z = a.z_score;
            altman_zone = a.zone.clone();
            let score = match a.zone.as_str() {
                "SAFE" => 10.0,
                "GRAY" => 55.0,
                "DISTRESS" => { distressed = true; 95.0 }
                _ => 50.0,
            };
            let w = 25.0;
            components.push(FactorComponent {
                name: "Altman Z".to_string(),
                value: format!("Z {:.2} ({})", a.z_score, a.zone),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return RiskSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            risk_label: "NO_DATA".to_string(),
            note: "need at least one of VOLE / BETA / LIQ / SHRT / ALTZ cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let label = if distressed { "DISTRESSED" }
                else if composite >= 75.0 { "HIGH_RISK" }
                else if composite >= 55.0 { "ELEVATED" }
                else if composite >= 35.0 { "MODERATE" }
                else { "LOW_RISK" };

    RiskSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        realized_vol_pct,
        beta_1y,
        liquidity_tier,
        short_percent_of_float,
        days_to_cover,
        altman_z,
        altman_zone,
        composite_score: composite,
        risk_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

/// INSSTRK — Insider streak detector from cached Form 4 trades.
pub fn compute_insstrk_snapshot(
    symbol: &str,
    as_of: &str,
    trades: &[InsiderTrade],
    window_days: i32,
) -> InsiderStreakSnapshot {
    let sym = symbol.to_uppercase();

    let as_of_days = parse_yyyy_mm_dd_to_days(as_of);
    let window_floor_days = as_of_days.map(|d| d - window_days as i64);

    // Filter to window.
    let mut filtered: Vec<&InsiderTrade> = trades.iter()
        .filter(|t| {
            let txn_days = parse_yyyy_mm_dd_to_days(&t.transaction_date);
            match (txn_days, window_floor_days) {
                (Some(td), Some(floor)) => td >= floor,
                _ => true,
            }
        })
        .collect();

    if filtered.is_empty() {
        return InsiderStreakSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            window_days,
            streak_label: "NONE".to_string(),
            note: "no insider trades within window".to_string(),
            ..Default::default()
        };
    }

    // Sort chronologically (oldest first) so streaks read naturally.
    filtered.sort_by(|a, b| a.transaction_date.cmp(&b.transaction_date));

    // Group by insider name.
    use std::collections::BTreeMap;
    let mut per_insider: BTreeMap<String, Vec<&InsiderTrade>> = BTreeMap::new();
    for t in &filtered {
        per_insider.entry(t.reporting_name.clone()).or_default().push(*t);
    }

    let unique_insiders = per_insider.len();
    let mut rows: Vec<InsiderStreakRow> = Vec::new();
    let mut buy_streak_count = 0usize;
    let mut sell_streak_count = 0usize;
    let mut longest_buy_streak = 0usize;
    let mut longest_sell_streak = 0usize;
    let mut net_buy_value_usd = 0.0;
    let mut net_sell_value_usd = 0.0;

    for (name, ts) in &per_insider {
        // Classify each trade BUY/SELL/OTHER from transaction_type or acquisition_disposition.
        let dir_of = |t: &InsiderTrade| -> &'static str {
            let tt = t.transaction_type.to_uppercase();
            if tt.starts_with("P") || tt.contains("PURCHASE") { return "BUY"; }
            if tt.starts_with("S") || tt.contains("SALE") { return "SELL"; }
            if t.acquisition_disposition.to_uppercase() == "A" { return "BUY"; }
            if t.acquisition_disposition.to_uppercase() == "D" { return "SELL"; }
            "OTHER"
        };

        // Longest consecutive run of same direction (BUY or SELL only, OTHER breaks).
        let mut longest_run: usize = 0;
        let mut longest_dir: &'static str = "MIXED";
        let mut cur_run: usize = 0;
        let mut cur_dir: &'static str = "";
        for t in ts {
            let d = dir_of(t);
            if d == "OTHER" {
                cur_run = 0;
                cur_dir = "";
                continue;
            }
            if d == cur_dir { cur_run += 1; }
            else { cur_run = 1; cur_dir = d; }
            if cur_run > longest_run {
                longest_run = cur_run;
                longest_dir = cur_dir;
            }
        }

        // Net signed totals for this insider in window.
        let mut net_value = 0.0;
        let mut net_shares = 0.0;
        let mut has_buy = false;
        let mut has_sell = false;
        for t in ts {
            let d = dir_of(t);
            if d == "BUY" { net_value += t.value_usd; net_shares += t.shares; has_buy = true; }
            else if d == "SELL" { net_value -= t.value_usd; net_shares -= t.shares; has_sell = true; }
        }

        let mixed = has_buy && has_sell;
        let row_dir = if mixed { "MIXED".to_string() }
                      else if has_buy { "BUY".to_string() }
                      else if has_sell { "SELL".to_string() }
                      else { "OTHER".to_string() };

        if row_dir == "BUY" && longest_run >= 2 { buy_streak_count += 1; }
        if row_dir == "SELL" && longest_run >= 2 { sell_streak_count += 1; }
        if longest_dir == "BUY" && longest_run > longest_buy_streak { longest_buy_streak = longest_run; }
        if longest_dir == "SELL" && longest_run > longest_sell_streak { longest_sell_streak = longest_run; }
        if row_dir == "BUY" { net_buy_value_usd += net_value.max(0.0); }
        if row_dir == "SELL" { net_sell_value_usd += (-net_value).max(0.0); }

        let first_date = ts.first().map(|t| t.transaction_date.clone()).unwrap_or_default();
        let latest_date = ts.last().map(|t| t.transaction_date.clone()).unwrap_or_default();

        rows.push(InsiderStreakRow {
            insider_name: name.clone(),
            streak_direction: row_dir,
            consecutive_events: longest_run,
            net_value_usd: net_value,
            net_shares,
            first_date,
            latest_date,
        });
    }

    // Sort rows: buys first, then by longest streak desc.
    rows.sort_by(|a, b| {
        let ka = match a.streak_direction.as_str() { "BUY" => 0, "SELL" => 1, "MIXED" => 2, _ => 3 };
        let kb = match b.streak_direction.as_str() { "BUY" => 0, "SELL" => 1, "MIXED" => 2, _ => 3 };
        ka.cmp(&kb).then(b.consecutive_events.cmp(&a.consecutive_events))
    });

    let label = if buy_streak_count >= 3 && longest_buy_streak >= 4 {
        "STRONG_ACCUMULATION"
    } else if sell_streak_count >= 3 && longest_sell_streak >= 4 {
        "STRONG_DISTRIBUTION"
    } else if buy_streak_count >= 2 && sell_streak_count >= 2 {
        "MIXED"
    } else if buy_streak_count >= 2 {
        "ACCUMULATION"
    } else if sell_streak_count >= 2 {
        "DISTRIBUTION"
    } else if buy_streak_count > 0 || sell_streak_count > 0 {
        "MIXED"
    } else {
        "NONE"
    };

    InsiderStreakSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        window_days,
        unique_insiders,
        buy_streak_count,
        sell_streak_count,
        longest_buy_streak,
        longest_sell_streak,
        net_buy_value_usd,
        net_sell_value_usd,
        streak_label: label.to_string(),
        rows,
        note: String::new(),
    }
}

/// COVG — Analyst coverage breadth + churn snapshot.
pub fn compute_covg_snapshot(
    symbol: &str,
    as_of: &str,
    price_target: Option<&PriceTarget>,
    recs: &[AnalystRecommendation],
    updm: Option<&UpdmSnapshot>,
) -> CoverageSnapshot {
    let sym = symbol.to_uppercase();
    let mut inputs_available = 0usize;

    let mut num_analysts = 0;
    let mut target_mean = 0.0;
    let mut target_low = 0.0;
    let mut target_high = 0.0;
    if let Some(pt) = price_target {
        num_analysts = pt.num_analysts;
        target_mean = pt.target_mean;
        target_low = pt.target_low;
        target_high = pt.target_high;
        if num_analysts > 0 || target_mean > 0.0 {
            inputs_available += 1;
        }
    }

    // Consensus distribution from latest AnalystRecommendation row (sorted chronologically).
    let mut sb = 0; let mut b = 0; let mut h = 0; let mut s = 0; let mut ss = 0;
    if !recs.is_empty() {
        let mut sorted = recs.to_vec();
        sorted.sort_by(|a, b| a.period.cmp(&b.period));
        if let Some(latest) = sorted.last() {
            sb = latest.strong_buy;
            b = latest.buy;
            h = latest.hold;
            s = latest.sell;
            ss = latest.strong_sell;
            if (sb + b + h + s + ss) > 0 {
                inputs_available += 1;
            }
        }
    }
    let total_recs = sb + b + h + s + ss;
    let bull_ratio = if total_recs > 0 { (sb + b) as f64 / total_recs as f64 } else { 0.0 };

    // UPDM — churn activity (upgrades/downgrades 90d).
    let mut upgrades_90d = 0usize;
    let mut downgrades_90d = 0usize;
    let mut net_90d = 0i32;
    if let Some(u) = updm {
        if u.total_actions > 0 {
            upgrades_90d = u.upgrades_90d;
            downgrades_90d = u.downgrades_90d;
            net_90d = u.net_90d;
            inputs_available += 1;
        }
    }
    let churn_90d = upgrades_90d + downgrades_90d;

    if inputs_available == 0 {
        return CoverageSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            coverage_label: "NONE".to_string(),
            note: "need PriceTarget / AnalystRecommendations / UPDM cached".to_string(),
            ..Default::default()
        };
    }

    // Breadth — num_analysts normalized: ≥20 = 100, 0 = 0.
    let breadth = ((num_analysts as f64 / 20.0) * 100.0).clamp(0.0, 100.0);
    // Consensus — bull ratio × 100.
    let consensus = (bull_ratio * 100.0).clamp(0.0, 100.0);
    // Churn — net_90d centered at 50, ±5 per net action.
    let churn = (50.0 + (net_90d as f64) * 5.0).clamp(0.0, 100.0);

    let composite = breadth * 0.35 + consensus * 0.35 + churn * 0.30;

    let label = if num_analysts > 0 && num_analysts < 5 {
        "THIN"
    } else if net_90d >= 3 && breadth >= 70.0 {
        "EXPANDING"
    } else if net_90d <= -3 {
        "CONTRACTING"
    } else if composite >= 50.0 {
        "STABLE"
    } else if inputs_available == 0 {
        "NONE"
    } else {
        "STABLE"
    };

    CoverageSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        num_analysts,
        target_mean,
        target_low,
        target_high,
        consensus_strong_buy: sb,
        consensus_buy: b,
        consensus_hold: h,
        consensus_sell: s,
        consensus_strong_sell: ss,
        consensus_total: total_recs,
        consensus_bull_ratio: bull_ratio,
        upgrades_90d,
        downgrades_90d,
        net_90d,
        churn_90d,
        breadth_score: breadth,
        consensus_score: consensus,
        churn_score: churn,
        composite_score: composite,
        coverage_label: label.to_string(),
        inputs_available,
        note: String::new(),
    }
}

// ── ADR-123 Godel Parity Round 16 compute fns ──────────────────────────────

/// Simple quartile at `q ∈ [0,1]` via linear interpolation on a sorted slice.
/// Used by the Round 16 rank surfaces for p25 / p75 sector markers.
fn quantile_f64(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    if sorted.len() == 1 {
        return sorted[0];
    }
    let idx = q * (sorted.len() as f64 - 1.0);
    let lo = idx.floor() as usize;
    let hi = idx.ceil() as usize;
    if lo == hi {
        return sorted[lo];
    }
    let frac = idx - lo as f64;
    sorted[lo] * (1.0 - frac) + sorted[hi] * frac
}

/// Percentile-rank `value` vs `others` using the
/// `(below + 0.5 × equal) / total × 100` midrank convention.
/// When `higher_is_better == false`, the returned rank is inverted so that
/// a smaller input value yields a higher percentile (used by RRK where
/// composite is higher = riskier).
fn percentile_rank_score(value: f64, others: &[f64], higher_is_better: bool) -> f64 {
    let total = others.len() + 1;
    if total < 2 {
        return 50.0;
    }
    let (mut below, mut equal) = (0usize, 0usize);
    for &o in others {
        if (o - value).abs() < 1e-9 {
            equal += 1;
        } else if higher_is_better {
            if o < value { below += 1; }
        } else {
            if o > value { below += 1; }
        }
    }
    let raw = (below as f64 + 0.5 * equal as f64 + 0.5) / total as f64 * 100.0;
    raw.clamp(0.0, 100.0)
}

/// Standard 6-bucket rank label ladder for VRK / QRK.
fn rank_label_for_percentile(pct: f64) -> &'static str {
    if pct >= 90.0 { "TOP_DECILE" }
    else if pct >= 75.0 { "TOP_QUARTILE" }
    else if pct >= 50.0 { "ABOVE_MEDIAN" }
    else if pct >= 25.0 { "BELOW_MEDIAN" }
    else if pct >= 10.0 { "BOTTOM_QUARTILE" }
    else { "BOTTOM_DECILE" }
}

/// Risk-inverted rank label ladder for RRK (higher rank = safer).
fn risk_rank_label_for_percentile(pct: f64) -> &'static str {
    if pct >= 90.0 { "SAFEST_DECILE" }
    else if pct >= 75.0 { "SAFEST_QUARTILE" }
    else if pct >= 50.0 { "ABOVE_MEDIAN_SAFE" }
    else if pct >= 25.0 { "BELOW_MEDIAN_RISKY" }
    else if pct >= 10.0 { "BOTTOM_QUARTILE_RISKY" }
    else { "RISKIEST_DECILE" }
}

/// VRK — Value Rank vs sector peers.
///
/// Takes the subject's `ValueSnapshot` and a slice of peer snapshots
/// (caller filters to the same sector). Returns a percentile rank with the
/// standard 6-bucket label ladder. Higher percentile = better value.
pub fn compute_vrk_snapshot(
    symbol: &str,
    as_of: &str,
    subject: Option<&ValueSnapshot>,
    peers: &[&ValueSnapshot],
) -> ValueRankSnapshot {
    let subj = match subject {
        Some(s) if s.value_label != "NO_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return ValueRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No VAL snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.value_label != "NO_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    if peer_scores.len() < 3 {
        return ValueRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: subj.sector.clone(),
            composite_score: subj.composite_score,
            peers_considered: peer_scores.len(),
            peers_with_data: peer_scores.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} VAL peers in sector {} (need ≥3)",
                peer_scores.len(),
                subj.sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, true);
    // 1-based rank position: count peers strictly better than subject + 1.
    let better = peer_scores.iter().filter(|&&p| p > subj.composite_score).count();
    let rank_position = better + 1;
    let label = rank_label_for_percentile(pct);
    ValueRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: subj.sector.clone(),
        composite_score: subj.composite_score,
        peers_considered: peer_scores.len(),
        peers_with_data: peer_scores.len(),
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// QRK — Quality Rank vs sector peers.
///
/// `QualitySnapshot` does not carry sector — caller must supply it (typically
/// from `fundamentals::get_fundamentals(symbol).sector`), and peers must be
/// pre-filtered to the same sector.
pub fn compute_qrk_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&QualitySnapshot>,
    peers: &[&QualitySnapshot],
) -> QualityRankSnapshot {
    let subj = match subject {
        Some(s) if s.quality_label != "NO_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return QualityRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No QUAL snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.quality_label != "NO_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    if peer_scores.len() < 3 {
        return QualityRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            peers_considered: peer_scores.len(),
            peers_with_data: peer_scores.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} QUAL peers in sector {} (need ≥3)",
                peer_scores.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, true);
    let better = peer_scores.iter().filter(|&&p| p > subj.composite_score).count();
    let label = rank_label_for_percentile(pct);
    QualityRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        peers_considered: peer_scores.len(),
        peers_with_data: peer_scores.len(),
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// RRK — Risk Rank vs sector peers.
///
/// Percentile rank is *inverted* relative to VRK/QRK: RISK composite is
/// higher = riskier, so this surface treats a **lower** composite as **better**
/// and reports "higher percentile = safer." Label ladder uses
/// SAFEST_DECILE..RISKIEST_DECILE phrasing so the inversion is explicit.
pub fn compute_rrk_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&RiskSnapshot>,
    peers: &[&RiskSnapshot],
) -> RiskRankSnapshot {
    let subj = match subject {
        Some(s) if s.risk_label != "NO_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return RiskRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No RISK snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.risk_label != "NO_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    if peer_scores.len() < 3 {
        return RiskRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            peers_considered: peer_scores.len(),
            peers_with_data: peer_scores.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} RISK peers in sector {} (need ≥3)",
                peer_scores.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // INVERSION: higher_is_better = false because RISK composite is higher = riskier.
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, false);
    // 1-based: rank position counted by how many peers are SAFER (lower composite).
    let safer = peer_scores.iter().filter(|&&p| p < subj.composite_score).count();
    let label = risk_rank_label_for_percentile(pct);
    RiskRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        peers_considered: peer_scores.len(),
        peers_with_data: peer_scores.len(),
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: safer + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// Compute 3-year EPS CAGR from a `FinancialStatements`.
/// Prefers annual rows (latest[0] vs latest[3] → 3y CAGR). Returns
/// `(latest_eps, earliest_eps, years_used, cagr_pct)` where `cagr_pct` is
/// `f64::NAN` if the sign-rule rejects the series.
fn eps_cagr_3y_from_statements(statements: &FinancialStatements) -> (f64, f64, usize, f64) {
    let annuals = &statements.income_annual;
    if annuals.len() < 4 {
        return (0.0, 0.0, 0, f64::NAN);
    }
    // Rows are assumed newest-first per the Finnhub fetcher convention.
    let latest = annuals[0].eps;
    let earliest = annuals[3].eps;
    let years = 3usize;
    // CAGR only valid when both endpoints are strictly positive.
    if latest > 0.0 && earliest > 0.0 {
        let cagr = ((latest / earliest).powf(1.0 / years as f64) - 1.0) * 100.0;
        (latest, earliest, years, cagr)
    } else if latest.is_finite() && earliest.is_finite() && earliest.abs() > 1e-9 {
        // Degrade gracefully to a linear annualised growth when signs cross:
        // this is the "CAGR_NEGATIVE" path — the snapshot label captures it.
        let linear = (latest - earliest) / earliest.abs() / years as f64 * 100.0;
        (latest, earliest, years, linear)
    } else {
        (latest, earliest, years, f64::NAN)
    }
}

/// RELEPSGR — Relative 3-year EPS CAGR vs sector median.
///
/// Computes the subject's 3y EPS CAGR and the median CAGR of the peer slice,
/// then labels the subject relative to the sector median. Labels:
/// FAR_ABOVE (≥ +15pp), ABOVE (≥ +5pp), INLINE (within ±5pp), BELOW (≤ -5pp),
/// FAR_BELOW (≤ -15pp), CAGR_NEGATIVE (sign-crossed subject EPS),
/// NO_DATA (insufficient annual rows or empty peer set).
pub fn compute_relepsgr_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&FinancialStatements>,
    peer_statements: &[(String, FinancialStatements)],
) -> RelativeEpsGrowthSnapshot {
    let subj = match subject {
        Some(s) if s.income_annual.len() >= 4 => s,
        _ => {
            return RelativeEpsGrowthSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                relative_label: "NO_DATA".into(),
                note: "Subject has < 4 annual income rows".into(),
                ..Default::default()
            };
        }
    };
    let (latest, earliest, years, subj_cagr) = eps_cagr_3y_from_statements(subj);
    let mut peer_cagrs: Vec<f64> = Vec::new();
    for (_, st) in peer_statements {
        if st.income_annual.len() < 4 {
            continue;
        }
        let (_, _, _, c) = eps_cagr_3y_from_statements(st);
        if c.is_finite() {
            peer_cagrs.push(c);
        }
    }
    let peers_considered = peer_statements.len();
    let peers_with_data = peer_cagrs.len();
    if peer_cagrs.len() < 3 {
        return RelativeEpsGrowthSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_eps: latest,
            earliest_eps: earliest,
            years_used: years,
            symbol_cagr_pct: if subj_cagr.is_finite() { subj_cagr } else { 0.0 },
            peers_considered,
            peers_with_data,
            relative_label: "NO_DATA".into(),
            note: format!("Only {} peers with ≥4 annual rows (need ≥3)", peer_cagrs.len()),
            ..Default::default()
        };
    }
    let mut sorted = peer_cagrs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    if !subj_cagr.is_finite() || latest <= 0.0 || earliest <= 0.0 {
        return RelativeEpsGrowthSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_eps: latest,
            earliest_eps: earliest,
            years_used: years,
            symbol_cagr_pct: if subj_cagr.is_finite() { subj_cagr } else { 0.0 },
            peers_considered,
            peers_with_data,
            sector_median_cagr_pct: median,
            sector_p25_cagr_pct: p25,
            sector_p75_cagr_pct: p75,
            relative_label: "CAGR_NEGATIVE".into(),
            note: "Subject EPS crosses zero; using linear proxy".into(),
            ..Default::default()
        };
    }
    let gap = subj_cagr - median;
    let label = if gap >= 15.0 { "FAR_ABOVE" }
        else if gap >= 5.0 { "ABOVE" }
        else if gap >= -5.0 { "INLINE" }
        else if gap >= -15.0 { "BELOW" }
        else { "FAR_BELOW" };
    RelativeEpsGrowthSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        latest_eps: latest,
        earliest_eps: earliest,
        years_used: years,
        symbol_cagr_pct: subj_cagr,
        peers_considered,
        peers_with_data,
        sector_median_cagr_pct: median,
        sector_p25_cagr_pct: p25,
        sector_p75_cagr_pct: p75,
        gap_to_median_pp: gap,
        relative_label: label.into(),
        note: String::new(),
    }
}

/// Locate the index of the first bar with `date >= target_date` in a
/// newest-first HP bar slice. Returns `None` if no such bar exists.
fn find_t0_index_newest_first(bars: &[HistoricalPriceRow], target_date: &str) -> Option<usize> {
    // Scan from oldest to newest (reverse iteration) and return the first
    // bar that is on-or-after the target. "newest-first" means bars[0] is
    // the most recent trading day.
    let mut best: Option<usize> = None;
    for (i, b) in bars.iter().enumerate() {
        if b.date.as_str() >= target_date {
            best = Some(i);
        } else {
            break;
        }
    }
    best
}

/// PEAD — Post-Earnings-Announcement Drift snapshot.
///
/// For each surprise row, locate `T0` in the HP bar slice (first trading day
/// at or after the announcement date), then compute forward drift over 1 / 3 /
/// 5 / 10 trading days. Averages over all successfully-matched events.
/// Returns INSUFFICIENT_DATA if fewer than 3 events match.
pub fn compute_pead_snapshot(
    symbol: &str,
    as_of: &str,
    surprises: &[EarningsSurprise],
    bars_newest_first: &[HistoricalPriceRow],
) -> PeadSnapshot {
    let num_events = surprises.len();
    if num_events == 0 || bars_newest_first.len() < 11 {
        return PeadSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            num_events,
            events_used: 0,
            drift_direction_label: "INSUFFICIENT_DATA".into(),
            note: if num_events == 0 {
                "No earnings surprises cached".into()
            } else {
                format!("Need ≥11 HP bars, have {}", bars_newest_first.len())
            },
            ..Default::default()
        };
    }
    let mut rows: Vec<PeadEventRow> = Vec::new();
    let mut beat_drifts_5d: Vec<f64> = Vec::new();
    let mut miss_drifts_5d: Vec<f64> = Vec::new();
    let mut all_1d: Vec<f64> = Vec::new();
    let mut all_3d: Vec<f64> = Vec::new();
    let mut all_5d: Vec<f64> = Vec::new();
    let mut all_10d: Vec<f64> = Vec::new();
    for surprise in surprises {
        let t0_idx = match find_t0_index_newest_first(bars_newest_first, &surprise.date) {
            Some(i) => i,
            None => continue,
        };
        // drift_Nd: close(t0 - N days back in newest-first ordering) vs close(t0).
        // Because bars are newest-first, "N trading days forward" means a
        // *smaller* index. Subtract N from t0_idx.
        if t0_idx < 10 {
            continue;
        }
        let t0_close = bars_newest_first[t0_idx].close;
        if t0_close <= 0.0 {
            continue;
        }
        let drift = |n: usize| {
            let fwd = &bars_newest_first[t0_idx - n];
            (fwd.close / t0_close - 1.0) * 100.0
        };
        let d1 = drift(1);
        let d3 = drift(3);
        let d5 = drift(5);
        let d10 = drift(10);
        let classification = if surprise.surprise_pct > 2.0 {
            "BEAT"
        } else if surprise.surprise_pct < -2.0 {
            "MISS"
        } else {
            "INLINE"
        };
        match classification {
            "BEAT" => beat_drifts_5d.push(d5),
            "MISS" => miss_drifts_5d.push(d5),
            _ => {}
        }
        all_1d.push(d1);
        all_3d.push(d3);
        all_5d.push(d5);
        all_10d.push(d10);
        rows.push(PeadEventRow {
            event_date: surprise.date.clone(),
            surprise_pct: surprise.surprise_pct,
            classification: classification.into(),
            drift_1d_pct: d1,
            drift_3d_pct: d3,
            drift_5d_pct: d5,
            drift_10d_pct: d10,
        });
    }
    let events_used = rows.len();
    if events_used < 3 {
        return PeadSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            num_events,
            events_used,
            drift_direction_label: "INSUFFICIENT_DATA".into(),
            note: format!("Matched only {} events to HP bars (need ≥3)", events_used),
            rows,
            ..Default::default()
        };
    }
    let mean = |v: &[f64]| if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / v.len() as f64 };
    let avg_1d = mean(&all_1d);
    let avg_3d = mean(&all_3d);
    let avg_5d = mean(&all_5d);
    let avg_10d = mean(&all_10d);
    let beat_5d = mean(&beat_drifts_5d);
    let miss_5d = mean(&miss_drifts_5d);
    // Sort rows newest-first (highest event_date string first) for stable display.
    let mut sorted_rows = rows.clone();
    sorted_rows.sort_by(|a, b| b.event_date.cmp(&a.event_date));
    let latest = sorted_rows.first().cloned().unwrap_or_default();
    let label = if avg_5d >= 2.0 { "DRIFT_UP" }
        else if avg_5d <= -2.0 { "DRIFT_DOWN" }
        else { "MIXED" };
    let display_rows: Vec<PeadEventRow> = sorted_rows.into_iter().take(8).collect();
    PeadSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        num_events,
        events_used,
        avg_drift_1d_pct: avg_1d,
        avg_drift_3d_pct: avg_3d,
        avg_drift_5d_pct: avg_5d,
        avg_drift_10d_pct: avg_10d,
        beat_event_drift_5d_pct: beat_5d,
        miss_event_drift_5d_pct: miss_5d,
        latest_event_date: latest.event_date.clone(),
        latest_event_surprise_pct: latest.surprise_pct,
        latest_event_drift_5d_pct: latest.drift_5d_pct,
        drift_direction_label: label.into(),
        rows: display_rows,
        note: String::new(),
    }
}

// ── ADR-124 Round 17 — rank surfaces + FQM + revenue growth ────────────────

/// Market-cap tier classifier (absolute dollar thresholds).
fn size_tier_label(market_cap: f64) -> &'static str {
    if market_cap >= 200_000_000_000.0 { "MEGA_CAP" }
    else if market_cap >= 10_000_000_000.0 { "LARGE_CAP" }
    else if market_cap >= 2_000_000_000.0 { "MID_CAP" }
    else if market_cap >= 300_000_000.0 { "SMALL_CAP" }
    else if market_cap > 0.0 { "MICRO_CAP" }
    else { "NO_DATA" }
}

/// SIZEF — Size Factor Rank vs sector peers.
///
/// Callers pass the subject's market cap + sector and a slice of
/// `(symbol, market_cap)` tuples for sector peers. Returns a percentile
/// rank (higher = larger) plus a tier label derived from absolute cap.
pub fn compute_sizef_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_market_cap: Option<f64>,
    peers: &[(String, f64)],
) -> SizeFactorSnapshot {
    let cap = match subject_market_cap {
        Some(c) if c > 0.0 => c,
        _ => {
            return SizeFactorSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                tier_label: "NO_DATA".into(),
                rank_label: "NO_DATA".into(),
                note: "No market cap on file for subject".into(),
                ..Default::default()
            };
        }
    };
    let tier = size_tier_label(cap);
    let peer_caps: Vec<f64> = peers
        .iter()
        .filter(|(_, c)| *c > 0.0)
        .map(|(_, c)| *c)
        .collect();
    if peer_caps.len() < 3 {
        return SizeFactorSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            market_cap: cap,
            log_market_cap: cap.ln(),
            tier_label: tier.into(),
            peers_considered: peer_caps.len(),
            peers_with_data: peer_caps.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} peers with market cap in sector {} (need ≥3)",
                peer_caps.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_caps.clone();
    sorted.push(cap);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(cap, &peer_caps, true);
    let better = peer_caps.iter().filter(|&&c| c > cap).count();
    let label = rank_label_for_percentile(pct);
    SizeFactorSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        market_cap: cap,
        log_market_cap: cap.ln(),
        tier_label: tier.into(),
        peers_considered: peer_caps.len(),
        peers_with_data: peer_caps.len(),
        sector_median_cap: median,
        sector_p25_cap: p25,
        sector_p75_cap: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// MOMF — Momentum Factor Rank vs sector peers.
///
/// `MomentumSnapshot` does not carry sector — caller must supply it and
/// pre-filter peers to the same sector.
pub fn compute_momf_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&MomentumSnapshot>,
    peers: &[&MomentumSnapshot],
) -> MomentumRankSnapshot {
    let subj = match subject {
        Some(s) if s.regime_label != "INSUFFICIENT_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return MomentumRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No MOMENTUM snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.regime_label != "INSUFFICIENT_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    if peer_scores.len() < 3 {
        return MomentumRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            peers_considered: peer_scores.len(),
            peers_with_data: peer_scores.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} MOMENTUM peers in sector {} (need ≥3)",
                peer_scores.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, true);
    let better = peer_scores.iter().filter(|&&p| p > subj.composite_score).count();
    let label = rank_label_for_percentile(pct);
    MomentumRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        peers_considered: peer_scores.len(),
        peers_with_data: peer_scores.len(),
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// PEADRANK — Post-Earnings Drift Rank vs sector peers.
///
/// Peers must have a valid PEAD snapshot (`drift_direction_label !=
/// "INSUFFICIENT_DATA"` and `events_used >= 3`). Higher percentile =
/// stronger positive drift.
pub fn compute_peadrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&PeadSnapshot>,
    peers: &[&PeadSnapshot],
) -> PeadRankSnapshot {
    let subj = match subject {
        Some(s) if s.drift_direction_label != "INSUFFICIENT_DATA" && s.events_used >= 3 => s,
        _ => {
            return PeadRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No valid PEAD snapshot for subject (need ≥3 events)".into(),
                ..Default::default()
            };
        }
    };
    let peer_drifts: Vec<f64> = peers
        .iter()
        .filter(|p| p.drift_direction_label != "INSUFFICIENT_DATA" && p.events_used >= 3)
        .map(|p| p.avg_drift_5d_pct)
        .collect();
    if peer_drifts.len() < 3 {
        return PeadRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            avg_drift_5d_pct: subj.avg_drift_5d_pct,
            peers_considered: peer_drifts.len(),
            peers_with_data: peer_drifts.len(),
            rank_label: "NO_DATA".into(),
            note: format!(
                "Only {} valid PEAD peers in sector {} (need ≥3)",
                peer_drifts.len(),
                sector
            ),
            ..Default::default()
        };
    }
    let mut sorted = peer_drifts.clone();
    sorted.push(subj.avg_drift_5d_pct);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.avg_drift_5d_pct, &peer_drifts, true);
    let better = peer_drifts.iter().filter(|&&d| d > subj.avg_drift_5d_pct).count();
    let label = rank_label_for_percentile(pct);
    PeadRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        avg_drift_5d_pct: subj.avg_drift_5d_pct,
        peers_considered: peer_drifts.len(),
        peers_with_data: peer_drifts.len(),
        sector_median_drift_5d_pct: median,
        sector_p25_drift_5d_pct: p25,
        sector_p75_drift_5d_pct: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// FQM — Fundamental Quality Meter.
///
/// One-layer composite over PTFS + MARGINS + ACRL (weights 40/30/30).
/// Deliberately excludes LEV so the score reflects **operational** quality
/// (does the business machine convert sales into durable cash?) rather than
/// capital-structure strength. A highly-levered business with elite margins
/// and strong cash conversion will FQM-high and QUAL-mid — that's the
/// intended divergence from ADR-122 QUAL.
pub fn compute_fqm_snapshot(
    symbol: &str,
    as_of: &str,
    piotroski: Option<&PiotroskiSnapshot>,
    margins: Option<&MarginsSnapshot>,
    accruals: Option<&AccrualsSnapshot>,
) -> FundamentalQualityMeterSnapshot {
    let sym = symbol.to_uppercase();
    let mut components: Vec<FactorComponent> = Vec::new();
    let mut total_weight = 0.0;
    let mut weighted_sum = 0.0;
    let mut inputs_available = 0i32;

    let mut piotroski_score = 0;
    let mut piotroski_label = String::new();
    let mut operating_margin_pct = 0.0;
    let mut margin_trend_label = String::new();
    let mut cash_conversion_pct = 0.0;
    let mut accruals_trend_label = String::new();

    // PTFS — weight 40.
    if let Some(p) = piotroski {
        if p.strength_label != "INSUFFICIENT_DATA" && !p.strength_label.is_empty() {
            piotroski_score = p.f_score;
            piotroski_label = p.strength_label.clone();
            let score = (p.f_score as f64 / 9.0 * 100.0).clamp(0.0, 100.0);
            let w = 40.0;
            components.push(FactorComponent {
                name: "Piotroski F".to_string(),
                value: format!("{}/9 ({})", p.f_score, p.strength_label),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // MARGINS — weight 30.
    if let Some(m) = margins {
        if m.quality_label != "INSUFFICIENT_DATA" && !m.quality_label.is_empty() {
            operating_margin_pct = m.latest_operating_margin_pct;
            margin_trend_label = m.overall_trend_label.clone();
            let mut score: f64 = match m.quality_label.as_str() {
                "HIGH" => 85.0,
                "MEDIUM" => 60.0,
                "LOW" => 30.0,
                _ => 50.0,
            };
            match m.overall_trend_label.as_str() {
                "EXPANDING" => score = (score + 10.0).min(100.0),
                "CONTRACTING" => score = (score - 10.0).max(0.0),
                _ => {}
            }
            let w = 30.0;
            components.push(FactorComponent {
                name: "Margins".to_string(),
                value: format!("{} op {:.1}% ({})", m.quality_label, m.latest_operating_margin_pct, m.overall_trend_label),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    // ACRL — weight 30.
    if let Some(ac) = accruals {
        if !ac.trend_label.is_empty() {
            accruals_trend_label = ac.trend_label.clone();
            cash_conversion_pct = ac.ttm_cash_conversion_pct;
            let mut score: f64 = match ac.trend_label.as_str() {
                "IMPROVING" => 80.0,
                "STABLE" => 60.0,
                "MIXED" => 50.0,
                "DETERIORATING" => 30.0,
                _ => 50.0,
            };
            if ac.ttm_cash_conversion_pct >= 100.0 {
                score = (score + 10.0).min(100.0);
            } else if ac.ttm_cash_conversion_pct < 50.0 && ac.ttm_cash_conversion_pct != 0.0 {
                score = (score - 10.0).max(0.0);
            }
            let w = 30.0;
            components.push(FactorComponent {
                name: "Accruals".to_string(),
                value: format!("{} ({:.0}% cash conv)", ac.trend_label, ac.ttm_cash_conversion_pct),
                score, weight: w, contribution: score * w / 100.0,
            });
            weighted_sum += score * w;
            total_weight += w;
            inputs_available += 1;
        }
    }

    if inputs_available == 0 || total_weight <= 0.0 {
        return FundamentalQualityMeterSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            operator_label: "NO_DATA".to_string(),
            note: "need at least one of PTFS / MARGINS / ACRL cached".to_string(),
            ..Default::default()
        };
    }

    let composite = (weighted_sum / total_weight).clamp(0.0, 100.0);
    let label = if composite >= 85.0 { "ELITE_OPERATOR" }
                else if composite >= 70.0 { "STRONG_OPERATOR" }
                else if composite >= 50.0 { "AVERAGE_OPERATOR" }
                else if composite >= 30.0 { "WEAK_OPERATOR" }
                else { "BROKEN_OPERATOR" };

    FundamentalQualityMeterSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        piotroski_score,
        piotroski_label,
        operating_margin_pct,
        margin_trend_label,
        cash_conversion_pct,
        accruals_trend_label,
        composite_score: composite,
        operator_label: label.to_string(),
        inputs_available,
        components,
        note: String::new(),
    }
}

/// Compute 3-year revenue CAGR from a `FinancialStatements`.
/// Returns `(latest_rev, earliest_rev, years_used, cagr_pct)`. CAGR is
/// `f64::NAN` if the sign rule rejects the series (revenue must both be
/// strictly positive — revenue rarely crosses zero, so NAN usually signals
/// missing data).
fn revenue_cagr_3y_from_statements(statements: &FinancialStatements) -> (f64, f64, usize, f64) {
    let annuals = &statements.income_annual;
    if annuals.len() < 4 {
        return (0.0, 0.0, 0, f64::NAN);
    }
    let latest = annuals[0].revenue;
    let earliest = annuals[3].revenue;
    let years = 3usize;
    if latest > 0.0 && earliest > 0.0 {
        let cagr = ((latest / earliest).powf(1.0 / years as f64) - 1.0) * 100.0;
        (latest, earliest, years, cagr)
    } else if latest.is_finite() && earliest.is_finite() && earliest.abs() > 1e-9 {
        let linear = (latest - earliest) / earliest.abs() / years as f64 * 100.0;
        (latest, earliest, years, linear)
    } else {
        (latest, earliest, years, f64::NAN)
    }
}

/// REVRANK — Relative Revenue Growth Rank.
///
/// Mirrors RELEPSGR but over `IncomeStatement.revenue` instead of EPS.
/// Label ladder: FAR_ABOVE (≥+15pp), ABOVE (≥+5pp), INLINE (±5pp),
/// BELOW (≤-5pp), FAR_BELOW (≤-15pp), CAGR_NEGATIVE (subject endpoints
/// non-positive), NO_DATA.
pub fn compute_revrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&FinancialStatements>,
    peer_statements: &[(String, FinancialStatements)],
) -> RevenueGrowthRankSnapshot {
    let subj = match subject {
        Some(s) if s.income_annual.len() >= 4 => s,
        _ => {
            return RevenueGrowthRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                relative_label: "NO_DATA".into(),
                note: "Subject has < 4 annual income rows".into(),
                ..Default::default()
            };
        }
    };
    let (latest, earliest, years, subj_cagr) = revenue_cagr_3y_from_statements(subj);
    let mut peer_cagrs: Vec<f64> = Vec::new();
    for (_, st) in peer_statements {
        if st.income_annual.len() < 4 {
            continue;
        }
        let (_, _, _, c) = revenue_cagr_3y_from_statements(st);
        if c.is_finite() {
            peer_cagrs.push(c);
        }
    }
    let peers_considered = peer_statements.len();
    let peers_with_data = peer_cagrs.len();
    if peer_cagrs.len() < 3 {
        return RevenueGrowthRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_revenue: latest,
            earliest_revenue: earliest,
            years_used: years,
            symbol_cagr_pct: if subj_cagr.is_finite() { subj_cagr } else { 0.0 },
            peers_considered,
            peers_with_data,
            relative_label: "NO_DATA".into(),
            note: format!("Only {} peers with ≥4 annual rows (need ≥3)", peer_cagrs.len()),
            ..Default::default()
        };
    }
    let mut sorted = peer_cagrs.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    if !subj_cagr.is_finite() || latest <= 0.0 || earliest <= 0.0 {
        return RevenueGrowthRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            latest_revenue: latest,
            earliest_revenue: earliest,
            years_used: years,
            symbol_cagr_pct: if subj_cagr.is_finite() { subj_cagr } else { 0.0 },
            peers_considered,
            peers_with_data,
            sector_median_cagr_pct: median,
            sector_p25_cagr_pct: p25,
            sector_p75_cagr_pct: p75,
            relative_label: "CAGR_NEGATIVE".into(),
            note: "Subject revenue crosses zero; using linear proxy".into(),
            ..Default::default()
        };
    }
    let gap = subj_cagr - median;
    let label = if gap >= 15.0 { "FAR_ABOVE" }
        else if gap >= 5.0 { "ABOVE" }
        else if gap >= -5.0 { "INLINE" }
        else if gap >= -15.0 { "BELOW" }
        else { "FAR_BELOW" };
    RevenueGrowthRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        latest_revenue: latest,
        earliest_revenue: earliest,
        years_used: years,
        symbol_cagr_pct: subj_cagr,
        peers_considered,
        peers_with_data,
        sector_median_cagr_pct: median,
        sector_p25_cagr_pct: p25,
        sector_p75_cagr_pct: p75,
        gap_to_median_pp: gap,
        relative_label: label.into(),
        note: String::new(),
    }
}

// ── ADR-125 Round 18 compute fns ───────────────────────────────────────────

/// Compute the debt-to-equity ratio for a `LeverageSnapshot`.
/// Returns `None` when equity is non-positive (shell / deficit), which is
/// handled by the LEVRANK surface as a special "NEGATIVE_EQUITY" bucket.
fn debt_to_equity_for(lev: &LeverageSnapshot) -> Option<f64> {
    if lev.total_equity > 0.0 { Some(lev.total_debt / lev.total_equity) } else { None }
}

/// LEVRANK — Leverage Rank vs sector peers.
///
/// Percentile-ranks the subject's D/E (from the cached `LeverageSnapshot`)
/// against peer snapshots pre-filtered to the same sector. Uses the
/// risk-inverted rank ladder (SAFEST_DECILE..RISKIEST_DECILE) because lower
/// D/E = safer. Negative-equity subjects get a dedicated label.
pub fn compute_levrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&LeverageSnapshot>,
    peers: &[&LeverageSnapshot],
) -> LeverageRankSnapshot {
    let subj = match subject {
        Some(s) => s,
        None => {
            return LeverageRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No LEV snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let subj_d2e = match debt_to_equity_for(subj) {
        Some(v) => v,
        None => {
            return LeverageRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                total_debt: subj.total_debt,
                total_equity: subj.total_equity,
                rank_label: "NEGATIVE_EQUITY".into(),
                note: "Subject has non-positive equity; D/E undefined".into(),
                ..Default::default()
            };
        }
    };
    let peer_d2es: Vec<f64> = peers
        .iter()
        .filter_map(|p| debt_to_equity_for(p))
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_d2es.len();
    if peer_d2es.len() < 3 {
        return LeverageRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            debt_to_equity: subj_d2e,
            total_debt: subj.total_debt,
            total_equity: subj.total_equity,
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!("Only {} LEV peers with positive equity in sector {} (need ≥3)", peer_d2es.len(), sector),
            ..Default::default()
        };
    }
    let mut sorted = peer_d2es.clone();
    sorted.push(subj_d2e);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // INVERSION: lower D/E = safer = higher rank.
    let pct = percentile_rank_score(subj_d2e, &peer_d2es, false);
    // rank_position counted by how many peers are SAFER (lower D/E).
    let safer = peer_d2es.iter().filter(|&&p| p < subj_d2e).count();
    let label = risk_rank_label_for_percentile(pct);
    LeverageRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        debt_to_equity: subj_d2e,
        total_debt: subj.total_debt,
        total_equity: subj.total_equity,
        peers_considered,
        peers_with_data,
        sector_median_d2e: median,
        sector_p25_d2e: p25,
        sector_p75_d2e: p75,
        percentile_rank: pct,
        rank_position: safer + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// OPERANK — Operating Quality Rank vs sector peers.
///
/// Percentile-ranks `MarginsSnapshot.latest_operating_margin_pct` within
/// the same sector. Higher margin = higher rank. Peers must be pre-filtered.
pub fn compute_operank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&MarginsSnapshot>,
    peers: &[&MarginsSnapshot],
) -> OperatingQualityRankSnapshot {
    let subj = match subject {
        Some(s) if s.periods_used > 0 => s,
        _ => {
            return OperatingQualityRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No MARGINS snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_margins: Vec<f64> = peers
        .iter()
        .filter(|p| p.periods_used > 0)
        .map(|p| p.latest_operating_margin_pct)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_margins.len();
    if peer_margins.len() < 3 {
        return OperatingQualityRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            operating_margin_pct: subj.latest_operating_margin_pct,
            margin_trend_label: subj.overall_trend_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!("Only {} MARGINS peers in sector {} (need ≥3)", peer_margins.len(), sector),
            ..Default::default()
        };
    }
    let mut sorted = peer_margins.clone();
    sorted.push(subj.latest_operating_margin_pct);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.latest_operating_margin_pct, &peer_margins, true);
    let better = peer_margins.iter().filter(|&&p| p > subj.latest_operating_margin_pct).count();
    let label = rank_label_for_percentile(pct);
    OperatingQualityRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        operating_margin_pct: subj.latest_operating_margin_pct,
        margin_trend_label: subj.overall_trend_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_margin_pct: median,
        sector_p25_margin_pct: p25,
        sector_p75_margin_pct: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// FQMRANK — Fundamental Quality Meter Rank vs sector peers.
///
/// Percentile-ranks `FundamentalQualityMeterSnapshot.composite_score` within
/// the same sector. Filters out peers with operator_label "NO_DATA".
pub fn compute_fqmrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&FundamentalQualityMeterSnapshot>,
    peers: &[&FundamentalQualityMeterSnapshot],
) -> FqmRankSnapshot {
    let subj = match subject {
        Some(s) if s.operator_label != "NO_DATA" && s.composite_score > 0.0 => s,
        _ => {
            return FqmRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No FQM snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| p.operator_label != "NO_DATA" && p.composite_score > 0.0)
        .map(|p| p.composite_score)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_scores.len();
    if peer_scores.len() < 3 {
        return FqmRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            operator_label: subj.operator_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!("Only {} FQM peers in sector {} (need ≥3)", peer_scores.len(), sector),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, true);
    let better = peer_scores.iter().filter(|&&p| p > subj.composite_score).count();
    let label = rank_label_for_percentile(pct);
    FqmRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        operator_label: subj.operator_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// LIQRANK — Liquidity Rank vs sector peers.
///
/// Percentile-ranks `LiquiditySnapshot.avg_daily_dollar_volume` within the
/// same sector. Higher ADV$ = deeper liquidity = higher rank. Filters out
/// peers with INSUFFICIENT_DATA tier.
pub fn compute_liqrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&LiquiditySnapshot>,
    peers: &[&LiquiditySnapshot],
) -> LiquidityRankSnapshot {
    let subj = match subject {
        Some(s) if s.liquidity_tier != "INSUFFICIENT_DATA" && s.avg_daily_dollar_volume > 0.0 => s,
        _ => {
            return LiquidityRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No LIQ snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_advs: Vec<f64> = peers
        .iter()
        .filter(|p| p.liquidity_tier != "INSUFFICIENT_DATA" && p.avg_daily_dollar_volume > 0.0)
        .map(|p| p.avg_daily_dollar_volume)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_advs.len();
    if peer_advs.len() < 3 {
        return LiquidityRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            avg_daily_dollar_volume: subj.avg_daily_dollar_volume,
            tier_label: subj.liquidity_tier.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "NO_DATA".into(),
            note: format!("Only {} LIQ peers in sector {} (need ≥3)", peer_advs.len(), sector),
            ..Default::default()
        };
    }
    let mut sorted = peer_advs.clone();
    sorted.push(subj.avg_daily_dollar_volume);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.avg_daily_dollar_volume, &peer_advs, true);
    let better = peer_advs.iter().filter(|&&p| p > subj.avg_daily_dollar_volume).count();
    let label = rank_label_for_percentile(pct);
    LiquidityRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        avg_daily_dollar_volume: subj.avg_daily_dollar_volume,
        tier_label: subj.liquidity_tier.clone(),
        peers_considered,
        peers_with_data,
        sector_median_dollar_volume: median,
        sector_p25_dollar_volume: p25,
        sector_p75_dollar_volume: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// SURPSTK — Earnings Surprise Streak snapshot.
///
/// Pure time-series stat over cached `EarningsSurprise` rows. Classifies each
/// row as BEAT / MISS / INLINE using a ±2% band around the estimate, then
/// counts consecutive streaks over the series (sorted newest-first). Emits
/// a streak-strength label from the beat rate + current streak. No sector.
pub fn compute_surpstk_snapshot(
    symbol: &str,
    as_of: &str,
    surprises: &[EarningsSurprise],
) -> EarningsSurpriseStreakSnapshot {
    if surprises.is_empty() {
        return EarningsSurpriseStreakSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            streak_label: "INSUFFICIENT_DATA".into(),
            note: "No EPS rows cached for subject".into(),
            ..Default::default()
        };
    }
    // Sort newest-first by date (lexical works for YYYY-MM-DD).
    let mut rows: Vec<&EarningsSurprise> = surprises.iter().collect();
    rows.sort_by(|a, b| b.date.cmp(&a.date));
    let classify = |s: &EarningsSurprise| -> &'static str {
        if s.surprise_pct >= 2.0 { "BEAT" }
        else if s.surprise_pct <= -2.0 { "MISS" }
        else { "INLINE" }
    };
    let mut beats = 0usize;
    let mut misses = 0usize;
    let mut inlines = 0usize;
    let mut sum_surprise = 0.0f64;
    for r in &rows {
        sum_surprise += r.surprise_pct;
        match classify(r) {
            "BEAT" => beats += 1,
            "MISS" => misses += 1,
            _ => inlines += 1,
        }
    }
    let total = rows.len();
    let beat_rate = beats as f64 / total as f64 * 100.0;
    let avg_surprise = sum_surprise / total as f64;
    // Current streak: starts at rows[0] (newest) and extends while label matches.
    let current_label = classify(rows[0]);
    let mut current_len = 1usize;
    for r in rows.iter().skip(1) {
        if classify(r) == current_label { current_len += 1; } else { break; }
    }
    // Longest streaks scanned across the full series.
    let mut longest_beat = 0usize;
    let mut longest_miss = 0usize;
    let mut run_beat = 0usize;
    let mut run_miss = 0usize;
    for r in &rows {
        match classify(r) {
            "BEAT" => {
                run_beat += 1;
                run_miss = 0;
                if run_beat > longest_beat { longest_beat = run_beat; }
            }
            "MISS" => {
                run_miss += 1;
                run_beat = 0;
                if run_miss > longest_miss { longest_miss = run_miss; }
            }
            _ => {
                run_beat = 0;
                run_miss = 0;
            }
        }
    }
    let streak_label = if total < 4 {
        "INSUFFICIENT_DATA"
    } else if beat_rate >= 75.0 && current_label == "BEAT" && current_len >= 3 {
        "HOT_STREAK"
    } else if beat_rate >= 60.0 {
        "BEAT_TREND"
    } else if beat_rate <= 25.0 && current_label == "MISS" && current_len >= 3 {
        "COLD_STREAK"
    } else if beat_rate <= 40.0 {
        "MISS_TREND"
    } else {
        "MIXED"
    };
    let latest = rows[0];
    EarningsSurpriseStreakSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        total_events: total,
        beats,
        misses,
        inlines,
        beat_rate_pct: beat_rate,
        current_streak_type: current_label.to_string(),
        current_streak_len: current_len,
        longest_beat_streak: longest_beat,
        longest_miss_streak: longest_miss,
        avg_surprise_pct: avg_surprise,
        latest_event_date: latest.date.clone(),
        latest_event_surprise_pct: latest.surprise_pct,
        latest_event_label: classify(latest).to_string(),
        streak_label: streak_label.to_string(),
        note: String::new(),
    }
}

// ── ADR-126 Round 19 compute fns ──────────────────────────────────────────

pub fn compute_dvdrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&DivgSnapshot>,
    peers: &[&DivgSnapshot],
) -> DividendGrowthRankSnapshot {
    let subj = match subject {
        Some(s) if s.trend_label != "NO_HISTORY" && !s.trend_label.is_empty() => s,
        _ => {
            return DividendGrowthRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No DIVG snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_cagr: Vec<f64> = peers
        .iter()
        .filter(|p| !p.symbol.eq_ignore_ascii_case(symbol))
        .filter(|p| p.trend_label != "NO_HISTORY" && !p.trend_label.is_empty())
        .map(|p| p.cagr_3y_pct)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_cagr.len();
    if peer_cagr.len() < 3 {
        return DividendGrowthRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            cagr_3y_pct: subj.cagr_3y_pct,
            consecutive_growth_years: subj.consecutive_growth_years,
            trend_label: subj.trend_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("Only {} DIVG peers with history in sector {} (need ≥3)", peer_cagr.len(), sector),
            ..Default::default()
        };
    }
    let mut sorted = peer_cagr.clone();
    sorted.push(subj.cagr_3y_pct);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.cagr_3y_pct, &peer_cagr, true);
    let better = peer_cagr.iter().filter(|&&p| p > subj.cagr_3y_pct).count();
    let label = rank_label_for_percentile(pct);
    DividendGrowthRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        cagr_3y_pct: subj.cagr_3y_pct,
        consecutive_growth_years: subj.consecutive_growth_years,
        trend_label: subj.trend_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_cagr_pct: median,
        sector_p25_cagr_pct: p25,
        sector_p75_cagr_pct: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_earmrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&EarmSnapshot>,
    peers: &[&EarmSnapshot],
) -> EarningsMomentumRankSnapshot {
    let subj = match subject {
        Some(s) if s.momentum_label != "INSUFFICIENT_DATA" && !s.momentum_label.is_empty() => s,
        _ => {
            return EarningsMomentumRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No EARM snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_scores: Vec<f64> = peers
        .iter()
        .filter(|p| !p.symbol.eq_ignore_ascii_case(symbol))
        .filter(|p| p.momentum_label != "INSUFFICIENT_DATA" && !p.momentum_label.is_empty())
        .map(|p| p.composite_score)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_scores.len();
    if peer_scores.len() < 3 {
        return EarningsMomentumRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            composite_score: subj.composite_score,
            momentum_label: subj.momentum_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("Only {} EARM peers with data in sector {} (need ≥3)", peer_scores.len(), sector),
            ..Default::default()
        };
    }
    let mut sorted = peer_scores.clone();
    sorted.push(subj.composite_score);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj.composite_score, &peer_scores, true);
    let better = peer_scores.iter().filter(|&&p| p > subj.composite_score).count();
    let label = rank_label_for_percentile(pct);
    EarningsMomentumRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        composite_score: subj.composite_score,
        momentum_label: subj.momentum_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_score: median,
        sector_p25: p25,
        sector_p75: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_updgrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject: Option<&UpdmSnapshot>,
    peers: &[&UpdmSnapshot],
) -> UpgradeDowngradeRankSnapshot {
    let subj = match subject {
        Some(s) if s.bias_label != "NO_COVERAGE" && !s.bias_label.is_empty() => s,
        _ => {
            return UpgradeDowngradeRankSnapshot {
                symbol: symbol.to_string(),
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "No UPDM snapshot cached for subject".into(),
                ..Default::default()
            };
        }
    };
    let peer_nets: Vec<f64> = peers
        .iter()
        .filter(|p| !p.symbol.eq_ignore_ascii_case(symbol))
        .filter(|p| p.bias_label != "NO_COVERAGE" && !p.bias_label.is_empty())
        .map(|p| p.net_90d as f64)
        .collect();
    let peers_considered = peers.len();
    let peers_with_data = peer_nets.len();
    if peer_nets.len() < 3 {
        return UpgradeDowngradeRankSnapshot {
            symbol: symbol.to_string(),
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            net_90d: subj.net_90d,
            bias_label: subj.bias_label.clone(),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("Only {} UPDM peers with coverage in sector {} (need ≥3)", peer_nets.len(), sector),
            ..Default::default()
        };
    }
    let subj_f = subj.net_90d as f64;
    let mut sorted = peer_nets.clone();
    sorted.push(subj_f);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj_f, &peer_nets, true);
    let better = peer_nets.iter().filter(|&&p| p > subj_f).count();
    let label = rank_label_for_percentile(pct);
    UpgradeDowngradeRankSnapshot {
        symbol: symbol.to_string(),
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        net_90d: subj.net_90d,
        bias_label: subj.bias_label.clone(),
        peers_considered,
        peers_with_data,
        sector_median_net_90d: median,
        sector_p25_net_90d: p25,
        sector_p75_net_90d: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

pub fn compute_gy_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GapYearlySnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return GapYearlySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            gap_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars for gap calc".into(),
            ..Default::default()
        };
    }
    // Caller passes newest-first or oldest-first; we want to scan the last
    // 252 sessions worth of "today's open vs yesterday's close" gaps. Sort by
    // date ascending (oldest first) so pairs (i-1, i) go in calendar order.
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let start = if sorted.len() > 253 { sorted.len() - 253 } else { 0 };
    let window = &sorted[start..];
    let bars_used = window.len();
    let mut gaps_total = 0usize;
    let mut gaps_up_2 = 0usize;
    let mut gaps_down_2 = 0usize;
    let mut gaps_up_5 = 0usize;
    let mut gaps_down_5 = 0usize;
    let mut gaps_up_10 = 0usize;
    let mut gaps_down_10 = 0usize;
    let mut sum_abs = 0.0f64;
    let mut largest_up = 0.0f64;
    let mut largest_up_date = String::new();
    let mut largest_down = 0.0f64;
    let mut largest_down_date = String::new();
    for i in 1..window.len() {
        let prev_close = window[i - 1].close;
        let open = window[i].open;
        if prev_close <= 0.0 || open <= 0.0 { continue; }
        let gap_pct = (open - prev_close) / prev_close * 100.0;
        if gap_pct.abs() < 0.01 { continue; } // treat <0.01% as no gap
        gaps_total += 1;
        sum_abs += gap_pct.abs();
        if gap_pct >= 2.0 { gaps_up_2 += 1; }
        if gap_pct <= -2.0 { gaps_down_2 += 1; }
        if gap_pct >= 5.0 { gaps_up_5 += 1; }
        if gap_pct <= -5.0 { gaps_down_5 += 1; }
        if gap_pct >= 10.0 { gaps_up_10 += 1; }
        if gap_pct <= -10.0 { gaps_down_10 += 1; }
        if gap_pct > largest_up {
            largest_up = gap_pct;
            largest_up_date = window[i].date.clone();
        }
        if gap_pct < largest_down {
            largest_down = gap_pct;
            largest_down_date = window[i].date.clone();
        }
    }
    let avg_abs = if gaps_total > 0 { sum_abs / gaps_total as f64 } else { 0.0 };
    // Gap-label ladder:
    // - EXPLOSIVE: any 10% gap OR ≥ 4 gaps at the 5% band
    // - GAPPY: ≥ 12 gaps at the 2% band OR ≥ 2 gaps at the 5% band
    // - SMOOTH: < 6 gaps at the 2% band
    // - NORMAL: anything between
    let gap_2_total = gaps_up_2 + gaps_down_2;
    let gap_5_total = gaps_up_5 + gaps_down_5;
    let gap_10_total = gaps_up_10 + gaps_down_10;
    let gap_label = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if gap_10_total >= 1 || gap_5_total >= 4 {
        "EXPLOSIVE"
    } else if gap_2_total >= 12 || gap_5_total >= 2 {
        "GAPPY"
    } else if gap_2_total < 6 {
        "SMOOTH"
    } else {
        "NORMAL"
    };
    GapYearlySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        gaps_total,
        gaps_up_2pct: gaps_up_2,
        gaps_down_2pct: gaps_down_2,
        gaps_up_5pct: gaps_up_5,
        gaps_down_5pct: gaps_down_5,
        gaps_up_10pct: gaps_up_10,
        gaps_down_10pct: gaps_down_10,
        largest_up_gap_pct: largest_up,
        largest_up_gap_date: largest_up_date,
        largest_down_gap_pct: largest_down,
        largest_down_gap_date: largest_down_date,
        avg_abs_gap_pct: avg_abs,
        gap_label: gap_label.to_string(),
        note: String::new(),
    }
}

pub fn compute_des_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DailyEventStreakSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return DailyEventStreakSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            streak_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars for streak calc".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let start = if sorted.len() > 253 { sorted.len() - 253 } else { 0 };
    let window = &sorted[start..];
    let bars_used = window.len();
    let mut up_days = 0usize;
    let mut down_days = 0usize;
    let mut flat_days = 0usize;
    let mut sum_up = 0.0f64;
    let mut sum_down = 0.0f64;
    let mut dirs: Vec<i8> = Vec::with_capacity(window.len());
    for i in 1..window.len() {
        let prev = window[i - 1].close;
        let cur = window[i].close;
        if prev <= 0.0 || cur <= 0.0 { dirs.push(0); continue; }
        let pct = (cur - prev) / prev * 100.0;
        if pct > 0.0 {
            up_days += 1;
            sum_up += pct;
            dirs.push(1);
        } else if pct < 0.0 {
            down_days += 1;
            sum_down += pct;
            dirs.push(-1);
        } else {
            flat_days += 1;
            dirs.push(0);
        }
    }
    let mut longest_up = 0usize;
    let mut longest_down = 0usize;
    let mut run_up = 0usize;
    let mut run_down = 0usize;
    for d in &dirs {
        match *d {
            1 => {
                run_up += 1;
                run_down = 0;
                if run_up > longest_up { longest_up = run_up; }
            }
            -1 => {
                run_down += 1;
                run_up = 0;
                if run_down > longest_down { longest_down = run_down; }
            }
            _ => {
                run_up = 0;
                run_down = 0;
            }
        }
    }
    // Current streak: trailing run at the end of `dirs`.
    let (current_type, current_len) = if let Some(last) = dirs.last().copied() {
        let mut len = 0usize;
        if last != 0 {
            for d in dirs.iter().rev() {
                if *d == last { len += 1; } else { break; }
            }
        }
        match last {
            1 => ("UP".to_string(), len),
            -1 => ("DOWN".to_string(), len),
            0 => ("FLAT".to_string(), 0usize),
            _ => ("NONE".to_string(), 0usize),
        }
    } else {
        ("NONE".to_string(), 0usize)
    };
    let total_directional = up_days + down_days;
    let up_day_rate = if total_directional > 0 {
        up_days as f64 / total_directional as f64 * 100.0
    } else { 0.0 };
    let avg_up = if up_days > 0 { sum_up / up_days as f64 } else { 0.0 };
    let avg_down = if down_days > 0 { sum_down / down_days as f64 } else { 0.0 };
    let streak_label = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if up_day_rate >= 60.0 && longest_up >= 5 {
        "STRONG_UPTREND"
    } else if up_day_rate >= 55.0 {
        "UPTREND_BIAS"
    } else if up_day_rate <= 40.0 && longest_down >= 5 {
        "STRONG_DOWNTREND"
    } else if up_day_rate <= 45.0 {
        "DOWNTREND_BIAS"
    } else {
        "NEUTRAL"
    };
    DailyEventStreakSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        current_streak_type: current_type,
        current_streak_len: current_len,
        longest_up_streak: longest_up,
        longest_down_streak: longest_down,
        up_days,
        down_days,
        flat_days,
        up_day_rate_pct: up_day_rate,
        avg_up_move_pct: avg_up,
        avg_down_move_pct: avg_down,
        streak_label: streak_label.to_string(),
        note: String::new(),
    }
}

// ── ADR-127 Round 20 compute fns ──────────────────────────────────────────

/// DVDYIELDRANK compute: sector percentile rank of the subject's dividend
/// yield. Non-payers (None or 0.0) are filtered so the cohort is
/// dividend-paying names only. Needs ≥3 peers with yield data.
pub fn compute_dvdyieldrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_yield_pct: Option<f64>,
    peers: &[(String, Option<f64>)],
) -> DividendYieldRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject_yield_pct {
        Some(y) if y > 0.0 && y.is_finite() => y,
        _ => {
            return DividendYieldRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "subject has no dividend yield (non-payer or missing data)".into(),
                ..Default::default()
            };
        }
    };
    let peer_y: Vec<f64> = peers.iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .filter_map(|(_, y)| y.filter(|v| *v > 0.0 && v.is_finite()))
        .collect();
    let peers_considered = peers.iter().filter(|(s, _)| !s.eq_ignore_ascii_case(symbol)).count();
    let peers_with_data = peer_y.len();
    if peers_with_data < 3 {
        return DividendYieldRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            dividend_yield_pct: subj,
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 dividend-paying sector peers, got {}", peers_with_data),
            ..Default::default()
        };
    }
    let mut sorted = peer_y.clone();
    sorted.push(subj);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    let pct = percentile_rank_score(subj, &peer_y, true);
    let better = peer_y.iter().filter(|&&p| p > subj).count();
    let label = rank_label_for_percentile(pct);
    DividendYieldRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        dividend_yield_pct: subj,
        peers_considered,
        peers_with_data,
        sector_median_yield_pct: median,
        sector_p25_yield_pct: p25,
        sector_p75_yield_pct: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// SHRANK compute: sector percentile rank of short_percent_of_float,
/// risk-inverted so a *lower* short interest earns a *higher* (safer) rank.
pub fn compute_shrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_short_pct: Option<f64>,
    peers: &[(String, Option<f64>)],
) -> ShortInterestRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject_short_pct {
        Some(s) if s.is_finite() && s >= 0.0 => s,
        _ => {
            return ShortInterestRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "subject missing short_percent_of_float".into(),
                ..Default::default()
            };
        }
    };
    let peer_s: Vec<f64> = peers.iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .filter_map(|(_, v)| v.filter(|x| x.is_finite() && *x >= 0.0))
        .collect();
    let peers_considered = peers.iter().filter(|(s, _)| !s.eq_ignore_ascii_case(symbol)).count();
    let peers_with_data = peer_s.len();
    if peers_with_data < 3 {
        return ShortInterestRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            short_pct_of_float: subj,
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 sector peers with short data, got {}", peers_with_data),
            ..Default::default()
        };
    }
    let mut sorted = peer_s.clone();
    sorted.push(subj);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // Risk-inverted: lower short = safer = higher percentile
    let pct = percentile_rank_score(subj, &peer_s, false);
    // For risk surfaces, rank_position counts peers who are safer (lower short)
    let safer = peer_s.iter().filter(|&&p| p < subj).count();
    let label = risk_rank_label_for_percentile(pct);
    ShortInterestRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        short_pct_of_float: subj,
        peers_considered,
        peers_with_data,
        sector_median_short_pct: median,
        sector_p25_short_pct: p25,
        sector_p75_short_pct: p75,
        percentile_rank: pct,
        rank_position: safer + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// ATRANN compute: pure symbol-local 14-period Wilder ATR annualized, with
/// volatility regime label. Uses the most recent 253 sessions from the HP
/// cache, sorted oldest-first.
pub fn compute_atrann_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AnnualizedAtrSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 15 {
        return AnnualizedAtrSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥15 bars for 14-period ATR warmup".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let start = if sorted.len() > 253 { sorted.len() - 253 } else { 0 };
    let window = &sorted[start..];
    let bars_used = window.len();
    // True range for each bar i (i>=1): max(high-low, |high-prev_close|, |low-prev_close|)
    let mut trs: Vec<f64> = Vec::with_capacity(window.len());
    for i in 1..window.len() {
        let h = window[i].high;
        let l = window[i].low;
        let pc = window[i - 1].close;
        if h <= 0.0 || l <= 0.0 || pc <= 0.0 { continue; }
        let tr = (h - l).max((h - pc).abs()).max((l - pc).abs());
        trs.push(tr);
    }
    if trs.len() < 14 {
        return AnnualizedAtrSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            regime_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} usable TR bars after filtering", trs.len()),
            ..Default::default()
        };
    }
    // Wilder smoothing: seed = mean of first 14, then ATR_i = (prev_ATR × 13 + TR_i) / 14
    let seed: f64 = trs[..14].iter().sum::<f64>() / 14.0;
    let mut atr = seed;
    for &tr in &trs[14..] {
        atr = (atr * 13.0 + tr) / 14.0;
    }
    let latest_close = window.last().map(|r| r.close).unwrap_or(0.0);
    let atr_pct = if latest_close > 0.0 { atr / latest_close * 100.0 } else { 0.0 };
    let atr_ann = atr_pct * (252.0f64).sqrt();
    let regime = if atr_ann < 15.0 {
        "LOW_VOL"
    } else if atr_ann < 30.0 {
        "NORMAL_VOL"
    } else if atr_ann < 60.0 {
        "HIGH_VOL"
    } else {
        "EXTREME_VOL"
    };
    AnnualizedAtrSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        atr14: atr,
        atr14_pct: atr_pct,
        atr_annualized_pct: atr_ann,
        regime_label: regime.into(),
        note: String::new(),
    }
}

/// DDHIST compute: pure symbol-local drawdown history stat. Scans the
/// window for the deepest peak-to-trough, the longest peak-to-recovery
/// duration, the count of 5% and 10% corrections, and the current drawdown.
pub fn compute_ddhist_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DrawdownHistorySnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 20 {
        return DrawdownHistorySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥20 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let start = if sorted.len() > 253 { sorted.len() - 253 } else { 0 };
    let window = &sorted[start..];
    let bars_used = window.len();
    let mut running_peak = window[0].close;
    let mut running_peak_idx = 0usize;
    let mut running_peak_date = window[0].date.clone();
    let mut max_dd_pct = 0.0f64;
    let mut max_dd_peak_date = String::new();
    let mut max_dd_trough_date = String::new();
    let mut longest_dd_days = 0usize;
    let mut current_dd_start: Option<(usize, String)> = None; // (idx, date) of current peak
    let mut corrections_5 = 0usize;
    let mut corrections_10 = 0usize;
    // Track per-correction state: a "correction" is a run from a local peak to a local trough with ≥5% or ≥10% decline.
    let mut in_correction = false;
    let mut correction_peak = window[0].close;
    for (i, bar) in window.iter().enumerate() {
        let c = bar.close;
        if c <= 0.0 { continue; }
        if c >= running_peak {
            // Recovered — close the current drawdown bucket.
            if let Some((peak_idx, _)) = &current_dd_start {
                let duration = i - peak_idx;
                if duration > longest_dd_days { longest_dd_days = duration; }
            }
            current_dd_start = None;
            running_peak = c;
            running_peak_idx = i;
            running_peak_date = bar.date.clone();
            // Close any open correction by measuring its depth.
            if in_correction {
                let depth = (c - correction_peak) / correction_peak * 100.0;
                let abs_depth = -depth;  // positive number
                if abs_depth >= 10.0 { corrections_10 += 1; }
                if abs_depth >= 5.0 { corrections_5 += 1; }
                in_correction = false;
            }
            correction_peak = c;
        } else {
            // In a drawdown.
            if current_dd_start.is_none() {
                current_dd_start = Some((running_peak_idx, running_peak_date.clone()));
            }
            let dd = (c - running_peak) / running_peak * 100.0;  // negative
            if dd < max_dd_pct {
                max_dd_pct = dd;
                max_dd_peak_date = running_peak_date.clone();
                max_dd_trough_date = bar.date.clone();
            }
            // Correction tracking: local-peak-to-trough.
            if c < correction_peak {
                in_correction = true;
            }
        }
    }
    // If we ended the window still in a drawdown, count its duration.
    if let Some((peak_idx, _)) = &current_dd_start {
        let duration = window.len().saturating_sub(*peak_idx);
        if duration > longest_dd_days { longest_dd_days = duration; }
    }
    // Close any still-open correction (open means we ended the window below the local peak).
    if in_correction {
        let last = window.last().map(|r| r.close).unwrap_or(correction_peak);
        let abs_depth = (correction_peak - last) / correction_peak * 100.0;
        if abs_depth >= 10.0 { corrections_10 += 1; }
        if abs_depth >= 5.0 { corrections_5 += 1; }
    }
    let latest = window.last().map(|r| r.close).unwrap_or(0.0);
    let current_dd = if latest > 0.0 && running_peak > 0.0 {
        (latest - running_peak) / running_peak * 100.0
    } else { 0.0 };
    let regime = if current_dd > -1.0 {
        "RECOVERING"
    } else if max_dd_pct > -10.0 {
        "SHALLOW"
    } else if max_dd_pct > -20.0 {
        "MEANINGFUL"
    } else if max_dd_pct > -35.0 {
        "SEVERE"
    } else {
        "CATASTROPHIC"
    };
    DrawdownHistorySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        max_drawdown_pct: max_dd_pct,
        max_drawdown_peak_date: max_dd_peak_date,
        max_drawdown_trough_date: max_dd_trough_date,
        longest_drawdown_days: longest_dd_days,
        corrections_5pct: corrections_5,
        corrections_10pct: corrections_10,
        current_drawdown_pct: current_dd,
        regime_label: regime.into(),
        note: String::new(),
    }
}

/// PRICEPERF compute: multi-horizon total return stat. Computes returns
/// over trailing 21, 63, 126, and 253 sessions plus YTD from the first
/// session of as_of's calendar year.
pub fn compute_priceperf_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> PricePerformanceSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return PricePerformanceSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            trend_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let bars_used = sorted.len();
    let latest = sorted.last().unwrap();
    let latest_close = latest.close;
    if latest_close <= 0.0 {
        return PricePerformanceSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            trend_label: "INSUFFICIENT_DATA".into(),
            note: "latest close not positive".into(),
            ..Default::default()
        };
    }
    let ret_at = |offset: usize| -> f64 {
        if sorted.len() > offset {
            let past = sorted[sorted.len() - 1 - offset].close;
            if past > 0.0 { (latest_close - past) / past * 100.0 } else { 0.0 }
        } else { 0.0 }
    };
    let ret_1m = ret_at(21);
    let ret_3m = ret_at(63);
    let ret_6m = ret_at(126);
    let ret_1y = ret_at(253);
    // YTD: find first bar with date.year == latest.date.year
    let year_prefix = latest.date.get(..4).unwrap_or("");
    let ytd_ret = if !year_prefix.is_empty() {
        let ytd_start = sorted.iter().find(|r| r.date.starts_with(year_prefix));
        match ytd_start {
            Some(start_bar) if start_bar.close > 0.0 => {
                (latest_close - start_bar.close) / start_bar.close * 100.0
            }
            _ => 0.0,
        }
    } else { 0.0 };
    let trend = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if ret_1y > 30.0 && ret_3m > 10.0 {
        "STRONG_BULL"
    } else if ret_1y > 10.0 || ret_3m > 5.0 {
        "BULL"
    } else if ret_1y < -30.0 && ret_3m < -10.0 {
        "STRONG_BEAR"
    } else if ret_1y < -10.0 || ret_3m < -5.0 {
        "BEAR"
    } else {
        "NEUTRAL"
    };
    PricePerformanceSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        ret_1m_pct: ret_1m,
        ret_3m_pct: ret_3m,
        ret_6m_pct: ret_6m,
        ret_ytd_pct: ytd_ret,
        ret_1y_pct: ret_1y,
        trend_label: trend.into(),
        note: String::new(),
    }
}

// ── ADR-128 Round 21 compute fns ──

/// BETARANK compute: sector percentile rank of Fundamentals.beta,
/// risk-inverted so a *lower* beta earns a *higher* (safer) rank.
pub fn compute_betarank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_beta: Option<f64>,
    peers: &[(String, Option<f64>)],
) -> BetaRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject_beta {
        Some(b) if b.is_finite() => b,
        _ => {
            return BetaRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "subject missing beta".into(),
                ..Default::default()
            };
        }
    };
    let peer_b: Vec<f64> = peers.iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .filter_map(|(_, v)| v.filter(|x| x.is_finite()))
        .collect();
    let peers_considered = peers.iter().filter(|(s, _)| !s.eq_ignore_ascii_case(symbol)).count();
    let peers_with_data = peer_b.len();
    if peers_with_data < 3 {
        return BetaRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            subject_beta: Some(subj),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 sector peers with beta, got {}", peers_with_data),
            ..Default::default()
        };
    }
    let mut sorted = peer_b.clone();
    sorted.push(subj);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // Risk-inverted: lower beta = safer = higher percentile
    let pct = percentile_rank_score(subj, &peer_b, false);
    let safer = peer_b.iter().filter(|&&p| p < subj).count();
    let label = risk_rank_label_for_percentile(pct);
    BetaRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        subject_beta: Some(subj),
        peers_considered,
        peers_with_data,
        sector_median_beta: median,
        sector_p25_beta: p25,
        sector_p75_beta: p75,
        percentile_rank: pct,
        rank_position: safer + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// PEGRANK compute: sector percentile rank of Fundamentals.peg_ratio,
/// value-inverted so a *lower* PEG (cheaper growth) earns a *higher* rank.
/// Filters out non-positive or non-finite PEG on both subject and peer sides.
pub fn compute_pegrank_snapshot(
    symbol: &str,
    as_of: &str,
    sector: &str,
    subject_peg: Option<f64>,
    peers: &[(String, Option<f64>)],
) -> PegRankSnapshot {
    let sym = symbol.to_uppercase();
    let subj = match subject_peg {
        Some(p) if p > 0.0 && p.is_finite() => p,
        _ => {
            return PegRankSnapshot {
                symbol: sym,
                as_of: as_of.to_string(),
                sector: sector.to_string(),
                rank_label: "NO_DATA".into(),
                note: "subject has no valid PEG (negative or missing)".into(),
                ..Default::default()
            };
        }
    };
    let peer_p: Vec<f64> = peers.iter()
        .filter(|(s, _)| !s.eq_ignore_ascii_case(symbol))
        .filter_map(|(_, v)| v.filter(|x| *x > 0.0 && x.is_finite()))
        .collect();
    let peers_considered = peers.iter().filter(|(s, _)| !s.eq_ignore_ascii_case(symbol)).count();
    let peers_with_data = peer_p.len();
    if peers_with_data < 3 {
        return PegRankSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            sector: sector.to_string(),
            subject_peg: Some(subj),
            peers_considered,
            peers_with_data,
            rank_label: "INSUFFICIENT_DATA".into(),
            note: format!("need ≥3 sector peers with positive PEG, got {}", peers_with_data),
            ..Default::default()
        };
    }
    let mut sorted = peer_p.clone();
    sorted.push(subj);
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted, 0.5);
    let p25 = quantile_f64(&sorted, 0.25);
    let p75 = quantile_f64(&sorted, 0.75);
    // Value-inverted: lower PEG = better value = higher percentile
    let pct = percentile_rank_score(subj, &peer_p, false);
    let better = peer_p.iter().filter(|&&p| p < subj).count();
    let label = rank_label_for_percentile(pct);
    PegRankSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        sector: sector.to_string(),
        subject_peg: Some(subj),
        peers_considered,
        peers_with_data,
        sector_median_peg: median,
        sector_p25_peg: p25,
        sector_p75_peg: p75,
        percentile_rank: pct,
        rank_position: better + 1,
        rank_label: label.into(),
        note: String::new(),
    }
}

/// FHIGHLOW compute: 52-week high/low distance stat over cached HP bars.
/// Takes the trailing 253 sessions, tracks max close + min close + dates,
/// computes distance-from-high/low and range position, and emits a
/// proximity label band.
pub fn compute_fhighlow_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> FiftyTwoWeekHighLowSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return FiftyTwoWeekHighLowSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            proximity_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    // Trailing 253 sessions only.
    let window: Vec<&&HistoricalPriceRow> = sorted.iter().rev().take(253).collect();
    let bars_used = window.len();
    let latest = *window[0];
    let latest_close = latest.close;
    if latest_close <= 0.0 {
        return FiftyTwoWeekHighLowSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            proximity_label: "INSUFFICIENT_DATA".into(),
            note: "latest close not positive".into(),
            ..Default::default()
        };
    }
    let mut high = latest_close;
    let mut high_date = latest.date.clone();
    let mut high_idx: usize = 0; // index from latest (0 = most recent)
    let mut low = latest_close;
    let mut low_date = latest.date.clone();
    let mut low_idx: usize = 0;
    for (i, row) in window.iter().enumerate() {
        if row.close > 0.0 {
            if row.close > high {
                high = row.close;
                high_date = row.date.clone();
                high_idx = i;
            }
            if row.close < low {
                low = row.close;
                low_date = row.date.clone();
                low_idx = i;
            }
        }
    }
    let pct_from_high = if high > 0.0 { (latest_close - high) / high * 100.0 } else { 0.0 };
    let pct_from_low = if low > 0.0 { (latest_close - low) / low * 100.0 } else { 0.0 };
    let range = high - low;
    let range_position = if range > 0.0 { (latest_close - low) / range * 100.0 } else { 50.0 };
    let proximity = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if range_position >= 98.0 {
        "AT_HIGH"
    } else if range_position >= 80.0 {
        "NEAR_HIGH"
    } else if range_position >= 20.0 {
        "MID_RANGE"
    } else if range_position >= 2.0 {
        "NEAR_LOW"
    } else {
        "AT_LOW"
    };
    FiftyTwoWeekHighLowSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        high_52w: high,
        high_52w_date: high_date,
        days_since_high: high_idx,
        low_52w: low,
        low_52w_date: low_date,
        days_since_low: low_idx,
        pct_from_high,
        pct_from_low,
        range_position_pct: range_position,
        proximity_label: proximity.into(),
        note: String::new(),
    }
}

/// RVCONE compute: multi-horizon annualized realized volatility over the
/// HP cache, plus a rolling 20d RV percentile cone label.
pub fn compute_rvcone_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RealizedVolConeSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 21 {
        return RealizedVolConeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            cone_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥21 bars for 20-session realized vol".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let bars_used = sorted.len();
    let latest_close = sorted.last().unwrap().close;
    if latest_close <= 0.0 {
        return RealizedVolConeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            cone_label: "INSUFFICIENT_DATA".into(),
            note: "latest close not positive".into(),
            ..Default::default()
        };
    }
    // Log returns.
    let mut log_rets: Vec<f64> = Vec::with_capacity(sorted.len());
    for w in sorted.windows(2) {
        let prev = w[0].close;
        let curr = w[1].close;
        if prev > 0.0 && curr > 0.0 {
            log_rets.push((curr / prev).ln());
        }
    }
    if log_rets.len() < 20 {
        return RealizedVolConeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            cone_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    // Annualized realized vol of trailing n returns, as percent.
    let ann_rv = |n: usize| -> f64 {
        if log_rets.len() < n { return 0.0; }
        let slice = &log_rets[log_rets.len() - n..];
        let mean: f64 = slice.iter().sum::<f64>() / n as f64;
        let var: f64 = slice.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n as f64;
        var.sqrt() * (252.0_f64).sqrt() * 100.0
    };
    let rv20 = ann_rv(20);
    let rv60 = ann_rv(60);
    let rv120 = ann_rv(120);
    let rv252 = ann_rv(252);
    // Rolling 20d RV distribution across the full return window.
    let mut rolling20: Vec<f64> = Vec::new();
    if log_rets.len() >= 20 {
        for end in 20..=log_rets.len() {
            let slice = &log_rets[end - 20..end];
            let mean: f64 = slice.iter().sum::<f64>() / 20.0;
            let var: f64 = slice.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / 20.0;
            rolling20.push(var.sqrt() * (252.0_f64).sqrt() * 100.0);
        }
    }
    let (rv20_min, rv20_med, rv20_max, rv20_pct) = if !rolling20.is_empty() {
        let mut sorted_r = rolling20.clone();
        sorted_r.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let min = *sorted_r.first().unwrap();
        let max = *sorted_r.last().unwrap();
        let med = quantile_f64(&sorted_r, 0.5);
        // Percentile of latest rv20 within the historical rolling distribution.
        let others: Vec<f64> = rolling20.iter().take(rolling20.len() - 1).copied().collect();
        let pct = if others.is_empty() { 50.0 } else { percentile_rank_score(rv20, &others, true) };
        (min, med, max, pct)
    } else { (rv20, rv20, rv20, 50.0) };
    let cone = if rolling20.len() < 20 {
        "INSUFFICIENT_DATA"
    } else if rv20_pct >= 90.0 {
        "EXTREME"
    } else if rv20_pct >= 70.0 {
        "ELEVATED"
    } else if rv20_pct >= 30.0 {
        "TYPICAL"
    } else if rv20_pct >= 10.0 {
        "BELOW_AVG"
    } else {
        "COMPRESSED"
    };
    RealizedVolConeSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        rv20_pct: rv20,
        rv60_pct: rv60,
        rv120_pct: rv120,
        rv252_pct: rv252,
        rv20_min_pct: rv20_min,
        rv20_median_pct: rv20_med,
        rv20_max_pct: rv20_max,
        rv20_percentile: rv20_pct,
        cone_label: cone.into(),
        note: String::new(),
    }
}

/// CALPB compute: calendar-aligned period breakdowns over the HP cache.
/// Uses year-prefix / month-prefix string matching on `date` (assumes
/// ISO-8601 YYYY-MM-DD), like PRICEPERF's YTD shortcut.
pub fn compute_calpb_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> CalendarPeriodBreakdownSnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 2 {
        return CalendarPeriodBreakdownSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            momentum_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥2 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let bars_used = sorted.len();
    let latest = sorted.last().unwrap();
    let latest_close = latest.close;
    if latest_close <= 0.0 {
        return CalendarPeriodBreakdownSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            momentum_label: "INSUFFICIENT_DATA".into(),
            note: "latest close not positive".into(),
            ..Default::default()
        };
    }
    // Parse latest date as YYYY-MM-DD.
    let year: i32 = latest.date.get(..4).and_then(|s| s.parse().ok()).unwrap_or(0);
    let month: u32 = latest.date.get(5..7).and_then(|s| s.parse().ok()).unwrap_or(0);
    if year == 0 || month == 0 {
        return CalendarPeriodBreakdownSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used,
            momentum_label: "INSUFFICIENT_DATA".into(),
            note: "cannot parse latest bar date".into(),
            ..Default::default()
        };
    }
    let quarter = ((month - 1) / 3) + 1; // 1..=4
    let q_first_month = ((quarter - 1) * 3) + 1;
    // Helpers.
    let pct_from_first_in = |prefix: &str| -> f64 {
        if let Some(start) = sorted.iter().find(|r| r.date.starts_with(prefix)) {
            if start.close > 0.0 {
                return (latest_close - start.close) / start.close * 100.0;
            }
        }
        0.0
    };
    let full_period_return = |start_prefix: &str, end_prefix: &str| -> f64 {
        let first = sorted.iter().find(|r| r.date.starts_with(start_prefix));
        let last = sorted.iter().rev().find(|r| r.date.starts_with(end_prefix));
        match (first, last) {
            (Some(a), Some(b)) if a.close > 0.0 && b.close > 0.0 => (b.close - a.close) / a.close * 100.0,
            _ => 0.0,
        }
    };
    // MTD — bars with year-month prefix matching latest.
    let ym_prefix = format!("{:04}-{:02}", year, month);
    let mtd = pct_from_first_in(&ym_prefix);
    // QTD — bars from q_first_month of current year onwards.
    // Use inclusion filter across the 3 month-prefixes in current quarter.
    let qtd = {
        let q_prefixes: Vec<String> = (0..3)
            .map(|i| format!("{:04}-{:02}", year, q_first_month + i))
            .collect();
        let first = sorted.iter().find(|r| q_prefixes.iter().any(|p| r.date.starts_with(p)));
        match first {
            Some(bar) if bar.close > 0.0 => (latest_close - bar.close) / bar.close * 100.0,
            _ => 0.0,
        }
    };
    // YTD — first bar of current year to latest.
    let y_prefix = format!("{:04}", year);
    let ytd = pct_from_first_in(&y_prefix);
    // Prior quarter — full return over the quarter before the current one.
    let (prior_q_year, prior_q) = if quarter == 1 { (year - 1, 4u32) } else { (year, quarter - 1) };
    let prior_q_first_month = ((prior_q - 1) * 3) + 1;
    let prior_q_prefixes: Vec<String> = (0..3)
        .map(|i| format!("{:04}-{:02}", prior_q_year, prior_q_first_month + i))
        .collect();
    let prior_quarter = {
        let first = sorted.iter().find(|r| prior_q_prefixes.iter().any(|p| r.date.starts_with(p)));
        let last = sorted.iter().rev().find(|r| prior_q_prefixes.iter().any(|p| r.date.starts_with(p)));
        match (first, last) {
            (Some(a), Some(b)) if a.close > 0.0 && b.close > 0.0 => (b.close - a.close) / a.close * 100.0,
            _ => 0.0,
        }
    };
    // Prior year — full-year return of year-1.
    let prior_year_str = format!("{:04}", year - 1);
    let prior_year = full_period_return(&prior_year_str, &prior_year_str);
    // Momentum label: compare QTD vs prior_quarter.
    let momentum = if bars_used < 20 {
        "INSUFFICIENT_DATA"
    } else if qtd > prior_quarter + 5.0 && qtd > 0.0 {
        "ACCELERATING"
    } else if (qtd - prior_quarter).abs() <= 5.0 {
        "STEADY"
    } else if qtd < prior_quarter - 5.0 && qtd < 0.0 && prior_quarter < 0.0 {
        "DECELERATING"
    } else if qtd.signum() != prior_quarter.signum() && prior_quarter != 0.0 {
        "REVERSING"
    } else {
        "DECELERATING"
    };
    CalendarPeriodBreakdownSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        latest_close,
        mtd_pct: mtd,
        qtd_pct: qtd,
        ytd_pct: ytd,
        prior_quarter_pct: prior_quarter,
        prior_year_pct: prior_year,
        current_year: format!("{:04}", year),
        current_quarter: format!("Q{}", quarter),
        momentum_label: momentum.into(),
        note: String::new(),
    }
}

// ── ADR-129 Round 22 compute fns ──

/// Shared helper: collect trailing 253 bars sorted oldest-first and
/// compute log returns. Returns (sorted_bars, log_returns).
fn trailing_log_returns(bars: &[HistoricalPriceRow]) -> (Vec<&HistoricalPriceRow>, Vec<f64>) {
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    let mut log_rets: Vec<f64> = Vec::with_capacity(window.len());
    for w in window.windows(2) {
        let prev = w[0].close;
        let curr = w[1].close;
        if prev > 0.0 && curr > 0.0 {
            log_rets.push((curr / prev).ln());
        }
    }
    (window, log_rets)
}

/// RETSKEW compute: skewness of daily log returns over the trailing 253
/// sessions. Uses Fisher-Pearson (sample) skew with N denominator to match
/// RVCONE's stdev convention.
pub fn compute_retskew_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ReturnSkewnessSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return ReturnSkewnessSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            skew_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len() as f64;
    let mean: f64 = log_rets.iter().sum::<f64>() / n;
    let var: f64 = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    let stdev = var.sqrt();
    let skew = if stdev > 0.0 {
        let m3: f64 = log_rets.iter().map(|r| (r - mean).powi(3)).sum::<f64>() / n;
        m3 / stdev.powi(3)
    } else {
        0.0
    };
    let positive = log_rets.iter().filter(|&&r| r > 0.0).count() as f64;
    let positive_pct = (positive / n) * 100.0;
    let largest_up = log_rets.iter().cloned().fold(f64::NEG_INFINITY, f64::max) * 100.0;
    let largest_down = log_rets.iter().cloned().fold(f64::INFINITY, f64::min) * 100.0;
    let skew_label = if skew <= -1.0 {
        "STRONG_LEFT"
    } else if skew <= -0.3 {
        "LEFT"
    } else if skew < 0.3 {
        "SYMMETRIC"
    } else if skew < 1.0 {
        "RIGHT"
    } else {
        "STRONG_RIGHT"
    };
    ReturnSkewnessSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        mean_log_return: mean,
        stdev_log_return: stdev,
        skewness: skew,
        positive_return_pct: positive_pct,
        largest_up_pct: largest_up,
        largest_down_pct: largest_down,
        skew_label: skew_label.into(),
        note: String::new(),
    }
}

/// RETKURT compute: excess kurtosis of daily log returns over trailing 253
/// sessions. Counts 2-sigma and 3-sigma outliers for a non-parametric fat-
/// tail check alongside the moment-based number.
pub fn compute_retkurt_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ReturnKurtosisSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return ReturnKurtosisSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            kurt_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len() as f64;
    let mean: f64 = log_rets.iter().sum::<f64>() / n;
    let var: f64 = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / n;
    let stdev = var.sqrt();
    let excess = if stdev > 0.0 {
        let m4: f64 = log_rets.iter().map(|r| (r - mean).powi(4)).sum::<f64>() / n;
        (m4 / stdev.powi(4)) - 3.0
    } else {
        0.0
    };
    let (out2, out3) = if stdev > 0.0 {
        let mut c2 = 0usize;
        let mut c3 = 0usize;
        for r in &log_rets {
            let z = (r - mean).abs() / stdev;
            if z > 2.0 { c2 += 1; }
            if z > 3.0 { c3 += 1; }
        }
        (c2, c3)
    } else {
        (0, 0)
    };
    let out2_pct = (out2 as f64 / n) * 100.0;
    let kurt_label = if excess <= -0.5 {
        "PLATYKURTIC"
    } else if excess < 1.0 {
        "NORMAL"
    } else if excess < 3.0 {
        "MILD_FAT"
    } else if excess < 6.0 {
        "FAT"
    } else {
        "EXTREME_FAT"
    };
    ReturnKurtosisSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        mean_log_return: mean,
        stdev_log_return: stdev,
        excess_kurtosis: excess,
        outlier_2sigma_count: out2,
        outlier_3sigma_count: out3,
        outlier_2sigma_pct: out2_pct,
        kurt_label: kurt_label.into(),
        note: String::new(),
    }
}

/// TAILR compute: 95/5 and 99/1 tail ratios over trailing 253 sessions.
/// Non-parametric counterpart to RETSKEW.
pub fn compute_tailr_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> TailRatioSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return TailRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            bias_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let pct_returns: Vec<f64> = log_rets.iter().map(|r| r * 100.0).collect();
    let mut sorted = pct_returns.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let p95 = quantile_f64(&sorted, 0.95);
    let p05 = quantile_f64(&sorted, 0.05);
    let p99 = quantile_f64(&sorted, 0.99);
    let p01 = quantile_f64(&sorted, 0.01);
    let tail_ratio = if p05.abs() > f64::EPSILON { p95 / p05.abs() } else { 0.0 };
    let tail_ratio_99_01 = if p01.abs() > f64::EPSILON { p99 / p01.abs() } else { 0.0 };
    let bias_label = if tail_ratio <= 0.6 {
        "DOWNSIDE_HEAVY"
    } else if tail_ratio <= 0.85 {
        "SLIGHT_DOWNSIDE"
    } else if tail_ratio < 1.15 {
        "BALANCED"
    } else if tail_ratio < 1.4 {
        "SLIGHT_UPSIDE"
    } else {
        "UPSIDE_HEAVY"
    };
    TailRatioSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        pct_95_return: p95,
        pct_05_return: p05,
        pct_99_return: p99,
        pct_01_return: p01,
        tail_ratio,
        tail_ratio_99_01,
        bias_label: bias_label.into(),
        note: String::new(),
    }
}

/// RUNLEN compute: up/down day run length statistics over trailing 253
/// sessions. Uses sign of log return (0 → flat, included in neither run).
pub fn compute_runlen_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> RunLengthSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return RunLengthSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            trend_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let mut up_runs: Vec<usize> = Vec::new();
    let mut down_runs: Vec<usize> = Vec::new();
    let mut longest_up = 0usize;
    let mut longest_down = 0usize;
    let mut cur_up = 0usize;
    let mut cur_down = 0usize;
    for r in &log_rets {
        if *r > 0.0 {
            if cur_down > 0 {
                down_runs.push(cur_down);
                if cur_down > longest_down { longest_down = cur_down; }
                cur_down = 0;
            }
            cur_up += 1;
        } else if *r < 0.0 {
            if cur_up > 0 {
                up_runs.push(cur_up);
                if cur_up > longest_up { longest_up = cur_up; }
                cur_up = 0;
            }
            cur_down += 1;
        } else {
            if cur_up > 0 {
                up_runs.push(cur_up);
                if cur_up > longest_up { longest_up = cur_up; }
                cur_up = 0;
            }
            if cur_down > 0 {
                down_runs.push(cur_down);
                if cur_down > longest_down { longest_down = cur_down; }
                cur_down = 0;
            }
        }
    }
    // Tail: whichever run is still in progress is the "current" run.
    let current_run: i32 = if cur_up > 0 {
        up_runs.push(cur_up);
        if cur_up > longest_up { longest_up = cur_up; }
        cur_up as i32
    } else if cur_down > 0 {
        down_runs.push(cur_down);
        if cur_down > longest_down { longest_down = cur_down; }
        -(cur_down as i32)
    } else {
        0
    };
    let avg_up = if up_runs.is_empty() {
        0.0
    } else {
        up_runs.iter().sum::<usize>() as f64 / up_runs.len() as f64
    };
    let avg_down = if down_runs.is_empty() {
        0.0
    } else {
        down_runs.iter().sum::<usize>() as f64 / down_runs.len() as f64
    };
    let avg_run = (avg_up + avg_down) / 2.0;
    let longest_any = longest_up.max(longest_down) as f64;
    // Label combines avg run length and longest run length.
    let trend_label = if avg_run < 1.4 && longest_any < 4.0 {
        "CHOPPY"
    } else if avg_run < 1.7 && longest_any < 6.0 {
        "MIXED"
    } else if avg_run < 2.2 || longest_any < 8.0 {
        "TRENDING"
    } else {
        "STRONG_TRENDING"
    };
    RunLengthSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        avg_up_run: avg_up,
        avg_down_run: avg_down,
        longest_up_run: longest_up,
        longest_down_run: longest_down,
        up_runs_count: up_runs.len(),
        down_runs_count: down_runs.len(),
        current_run_length: current_run,
        trend_label: trend_label.into(),
        note: String::new(),
    }
}

/// DAYRANGE compute: average (high-low)/close ratio over 60d vs 252d
/// baseline. Compression ratio < 1 → tight; > 1 → expanded.
pub fn compute_dayrange_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DailyRangeSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    if window.len() < 20 {
        return DailyRangeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            range_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} bars", window.len()),
            ..Default::default()
        };
    }
    // Per-bar range ratio.
    let ratios: Vec<f64> = window.iter()
        .filter(|r| r.close > 0.0 && r.high >= r.low)
        .map(|r| ((r.high - r.low) / r.close) * 100.0)
        .collect();
    if ratios.len() < 20 {
        return DailyRangeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            range_label: "INSUFFICIENT_DATA".into(),
            note: "insufficient valid bars".into(),
            ..Default::default()
        };
    }
    let avg_all: f64 = ratios.iter().sum::<f64>() / ratios.len() as f64;
    let take60 = ratios.len().min(60);
    let slice60 = &ratios[ratios.len() - take60..];
    let avg60: f64 = slice60.iter().sum::<f64>() / take60 as f64;
    let latest = *ratios.last().unwrap();
    let widest = ratios.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let narrowest = ratios.iter().cloned().fold(f64::INFINITY, f64::min);
    let compression = if avg_all > f64::EPSILON { avg60 / avg_all } else { 1.0 };
    let range_label = if compression <= 0.75 {
        "TIGHT"
    } else if compression <= 0.9 {
        "COMPRESSED"
    } else if compression < 1.1 {
        "NORMAL"
    } else if compression < 1.35 {
        "EXPANDED"
    } else {
        "VERY_EXPANDED"
    };
    DailyRangeSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        avg_range_60_pct: avg60,
        avg_range_252_pct: avg_all,
        latest_range_pct: latest,
        compression_ratio: compression,
        widest_range_pct: widest,
        narrowest_range_pct: narrowest,
        range_label: range_label.into(),
        note: String::new(),
    }
}

// ── ADR-131 Round 23 computes (AUTOCOR / HURST / HITRATE / GLASYM / VOLRATIO) ──

/// Helper: autocorrelation of a return series at a given lag, computed
/// via the standard estimator `sum((r_t - mean)(r_{t-k} - mean)) /
/// sum((r_t - mean)^2)`. Returns 0.0 when the series is too short
/// (<= lag) or the denominator is 0.
fn acf_at_lag(rets: &[f64], lag: usize) -> f64 {
    if lag == 0 || rets.len() <= lag {
        return 0.0;
    }
    let n = rets.len() as f64;
    let mean: f64 = rets.iter().sum::<f64>() / n;
    let denom: f64 = rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>();
    if denom <= f64::EPSILON {
        return 0.0;
    }
    let num: f64 = (lag..rets.len())
        .map(|t| (rets[t] - mean) * (rets[t - lag] - mean))
        .sum();
    num / denom
}

/// AUTOCOR compute: autocorrelation of log returns at lags 1/5/10/20.
/// Labels from lag-1 ACF: strong mean-reversion, mean-reversion,
/// neutral, momentum, strong momentum.
pub fn compute_autocor_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> AutocorrelationSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return AutocorrelationSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let mean: f64 = log_rets.iter().sum::<f64>() / log_rets.len() as f64;
    let lag1 = acf_at_lag(&log_rets, 1);
    let lag5 = acf_at_lag(&log_rets, 5);
    let lag10 = acf_at_lag(&log_rets, 10);
    let lag20 = acf_at_lag(&log_rets, 20);
    let regime_label = if lag1 <= -0.15 {
        "STRONG_MEAN_REVERT"
    } else if lag1 <= -0.05 {
        "MEAN_REVERT"
    } else if lag1 < 0.05 {
        "NEUTRAL"
    } else if lag1 < 0.15 {
        "MOMENTUM"
    } else {
        "STRONG_MOMENTUM"
    };
    AutocorrelationSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        lag1_acf: lag1,
        lag5_acf: lag5,
        lag10_acf: lag10,
        lag20_acf: lag20,
        mean_log_return: mean,
        regime_label: regime_label.into(),
        note: String::new(),
    }
}

/// HURST compute: Hurst exponent via rescaled-range analysis.
/// Partitions the log return series into non-overlapping chunks of
/// size `scale`, computes R/S (range of cumulative deviations divided
/// by stdev) per chunk, averages across chunks, and regresses
/// `log(R/S_avg)` against `log(scale)`. The slope is H.
pub fn compute_hurst_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HurstSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 40 {
        return HurstSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            memory_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    // Build candidate scales: powers-of-two-ish, bounded so we always get
    // at least 2 chunks per scale.
    let n = log_rets.len();
    let candidate_scales: Vec<usize> = [8, 12, 16, 24, 32, 48, 64, 96, 128]
        .into_iter()
        .filter(|&s| s <= n / 2)
        .collect();
    if candidate_scales.len() < 2 {
        return HurstSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            memory_label: "INSUFFICIENT_DATA".into(),
            note: "too few R/S scales".into(),
            ..Default::default()
        };
    }

    let mut xs: Vec<f64> = Vec::new();
    let mut ys: Vec<f64> = Vec::new();
    for &scale in &candidate_scales {
        let num_chunks = n / scale;
        if num_chunks == 0 {
            continue;
        }
        let mut rs_vals: Vec<f64> = Vec::with_capacity(num_chunks);
        for c in 0..num_chunks {
            let start = c * scale;
            let end = start + scale;
            let slice = &log_rets[start..end];
            let mean: f64 = slice.iter().sum::<f64>() / scale as f64;
            // Cumulative deviations from the chunk mean.
            let mut cum: Vec<f64> = Vec::with_capacity(scale);
            let mut running = 0.0;
            for r in slice {
                running += r - mean;
                cum.push(running);
            }
            let max_c = cum.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
            let min_c = cum.iter().cloned().fold(f64::INFINITY, f64::min);
            let range = max_c - min_c;
            let var: f64 = slice.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / scale as f64;
            let sd = var.sqrt();
            if sd > f64::EPSILON && range > 0.0 {
                rs_vals.push(range / sd);
            }
        }
        if rs_vals.is_empty() {
            continue;
        }
        let avg_rs: f64 = rs_vals.iter().sum::<f64>() / rs_vals.len() as f64;
        if avg_rs > 0.0 {
            xs.push((scale as f64).ln());
            ys.push(avg_rs.ln());
        }
    }
    if xs.len() < 2 {
        return HurstSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            memory_label: "INSUFFICIENT_DATA".into(),
            note: "R/S regression had < 2 points".into(),
            ..Default::default()
        };
    }
    // OLS slope.
    let np = xs.len() as f64;
    let mean_x: f64 = xs.iter().sum::<f64>() / np;
    let mean_y: f64 = ys.iter().sum::<f64>() / np;
    let mut num = 0.0;
    let mut den = 0.0;
    for i in 0..xs.len() {
        let dx = xs[i] - mean_x;
        num += dx * (ys[i] - mean_y);
        den += dx * dx;
    }
    let h = if den > f64::EPSILON { num / den } else { 0.5 };
    let label = if h < 0.35 {
        "STRONG_MEAN_REVERT"
    } else if h < 0.45 {
        "MEAN_REVERT"
    } else if h < 0.55 {
        "RANDOM_WALK"
    } else if h < 0.65 {
        "PERSISTENT"
    } else {
        "STRONG_PERSISTENT"
    };
    HurstSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        hurst_exponent: h,
        scales_used: xs.len(),
        min_scale: *candidate_scales.iter().min().unwrap_or(&0),
        max_scale: *candidate_scales.iter().max().unwrap_or(&0),
        memory_label: label.into(),
        note: String::new(),
    }
}

/// HITRATE compute: share of positive-return bars over 5/20/60/252
/// trailing windows. Label combines the 20d and 60d hit rates: both
/// above 55% → BULLISH, both below 45% → BEARISH, otherwise NEUTRAL /
/// WEAK_BULLISH / WEAK_BEARISH based on the 20d alone.
pub fn compute_hitrate_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> HitRateSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return HitRateSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            hit_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    fn hit_over(rets: &[f64], take: usize) -> f64 {
        let start = rets.len().saturating_sub(take);
        let slice = &rets[start..];
        if slice.is_empty() { return 0.0; }
        let up = slice.iter().filter(|&&r| r > 0.0).count() as f64;
        up / slice.len() as f64
    }
    let h5 = hit_over(&log_rets, 5) * 100.0;
    let h20 = hit_over(&log_rets, 20) * 100.0;
    let h60 = hit_over(&log_rets, 60) * 100.0;
    let h252 = hit_over(&log_rets, 252) * 100.0;
    let up = log_rets.iter().filter(|&&r| r > 0.0).count();
    let down = log_rets.iter().filter(|&&r| r < 0.0).count();
    let flat = log_rets.len() - up - down;

    let label = if h20 >= 60.0 && h60 >= 55.0 {
        "BULLISH"
    } else if h20 >= 55.0 {
        "WEAK_BULLISH"
    } else if h20 <= 40.0 && h60 <= 45.0 {
        "BEARISH"
    } else if h20 <= 45.0 {
        "WEAK_BEARISH"
    } else {
        "NEUTRAL"
    };
    HitRateSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        hitrate_5d: h5,
        hitrate_20d: h20,
        hitrate_60d: h60,
        hitrate_252d: h252,
        up_days: up,
        down_days: down,
        flat_days: flat,
        hit_label: label.into(),
        note: String::new(),
    }
}

/// GLASYM compute: average and median magnitude of up vs down days.
/// Magnitudes are expressed as percent log returns × 100.
pub fn compute_glasym_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GainLossAsymmetrySnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 20 {
        return GainLossAsymmetrySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            asymmetry_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} valid log returns", log_rets.len()),
            ..Default::default()
        };
    }
    let mut ups: Vec<f64> = log_rets.iter().filter(|&&r| r > 0.0).map(|r| r * 100.0).collect();
    let mut downs: Vec<f64> = log_rets.iter().filter(|&&r| r < 0.0).map(|r| -r * 100.0).collect();
    if ups.is_empty() || downs.is_empty() {
        return GainLossAsymmetrySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            asymmetry_label: "INSUFFICIENT_DATA".into(),
            note: "all-up or all-down window".into(),
            up_days: ups.len(),
            down_days: downs.len(),
            ..Default::default()
        };
    }
    let avg_up: f64 = ups.iter().sum::<f64>() / ups.len() as f64;
    let avg_down: f64 = downs.iter().sum::<f64>() / downs.len() as f64;
    ups.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    downs.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_up = quantile_f64(&ups, 0.5);
    let median_down = quantile_f64(&downs, 0.5);
    let ratio = if avg_down > f64::EPSILON { avg_up / avg_down } else { 0.0 };
    let label = if ratio <= 0.75 {
        "DOWNSIDE_HEAVY"
    } else if ratio <= 0.9 {
        "SLIGHT_DOWNSIDE"
    } else if ratio < 1.1 {
        "BALANCED"
    } else if ratio < 1.3 {
        "SLIGHT_UPSIDE"
    } else {
        "UPSIDE_HEAVY"
    };
    GainLossAsymmetrySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        avg_up_pct: avg_up,
        avg_down_pct: avg_down,
        median_up_pct: median_up,
        median_down_pct: median_down,
        magnitude_ratio: ratio,
        up_days: ups.len(),
        down_days: downs.len(),
        asymmetry_label: label.into(),
        note: String::new(),
    }
}

/// VOLRATIO compute: up-day vs down-day volume summary over the
/// trailing 253-session window. Emits INSUFFICIENT_DATA when the HP
/// cache was populated without volume (all zeros).
pub fn compute_volratio_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VolumeRatioSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    if window.len() < 20 {
        return VolumeRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            flow_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} bars", window.len()),
            ..Default::default()
        };
    }
    let mut up_vols: Vec<f64> = Vec::new();
    let mut down_vols: Vec<f64> = Vec::new();
    for w in window.windows(2) {
        let prev = w[0].close;
        let curr = w[1].close;
        let vol = w[1].volume;
        if prev > 0.0 && curr > 0.0 && vol > 0.0 {
            let r = (curr / prev).ln();
            if r > 0.0 {
                up_vols.push(vol);
            } else if r < 0.0 {
                down_vols.push(vol);
            }
        }
    }
    if up_vols.is_empty() || down_vols.is_empty() {
        return VolumeRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            flow_label: "INSUFFICIENT_DATA".into(),
            note: "HP cache lacks volume or one side empty".into(),
            up_days: up_vols.len(),
            down_days: down_vols.len(),
            ..Default::default()
        };
    }
    let avg_up: f64 = up_vols.iter().sum::<f64>() / up_vols.len() as f64;
    let avg_down: f64 = down_vols.iter().sum::<f64>() / down_vols.len() as f64;
    let max_up = up_vols.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let max_down = down_vols.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let mut sorted_up = up_vols.clone();
    let mut sorted_down = down_vols.clone();
    sorted_up.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    sorted_down.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median_up = quantile_f64(&sorted_up, 0.5);
    let median_down = quantile_f64(&sorted_down, 0.5);
    let ratio = if avg_down > f64::EPSILON { avg_up / avg_down } else { 0.0 };
    let label = if ratio <= 0.8 {
        "DISTRIBUTION"
    } else if ratio <= 0.95 {
        "SLIGHT_DISTRIBUTION"
    } else if ratio < 1.05 {
        "NEUTRAL"
    } else if ratio < 1.25 {
        "SLIGHT_ACCUMULATION"
    } else {
        "ACCUMULATION"
    };
    VolumeRatioSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        avg_up_volume: avg_up,
        avg_down_volume: avg_down,
        median_up_volume: median_up,
        median_down_volume: median_down,
        up_down_volume_ratio: ratio,
        max_up_volume: max_up,
        max_down_volume: max_down,
        up_days: up_vols.len(),
        down_days: down_vols.len(),
        flow_label: label.into(),
        note: String::new(),
    }
}

// ── ADR-132 Round 24 computes (DRAWUP / GAPSTATS / VOLCLUSTER / CLOSEPLC / MRHL) ──

/// DRAWUP compute: trough-to-peak rally history over the trailing 253
/// sessions. Mirror of `compute_ddhist_snapshot` — flip peak↔trough and
/// drawdown↔drawup, keep everything else aligned.
pub fn compute_drawup_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DrawupHistorySnapshot {
    let sym = symbol.to_uppercase();
    if bars.len() < 20 {
        return DrawupHistorySnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: bars.len(),
            rally_label: "INSUFFICIENT_DATA".into(),
            note: "need ≥20 bars".into(),
            ..Default::default()
        };
    }
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let start = if sorted.len() > 253 { sorted.len() - 253 } else { 0 };
    let window = &sorted[start..];
    let bars_used = window.len();
    let mut running_trough = window[0].close;
    let mut running_trough_idx = 0usize;
    let mut running_trough_date = window[0].date.clone();
    let mut max_du_pct = 0.0f64;
    let mut max_du_trough_date = String::new();
    let mut max_du_peak_date = String::new();
    let mut longest_du_days = 0usize;
    let mut current_du_start: Option<(usize, String)> = None;
    let mut rallies_5 = 0usize;
    let mut rallies_10 = 0usize;
    let mut in_rally = false;
    let mut rally_trough = window[0].close;
    for (i, bar) in window.iter().enumerate() {
        let c = bar.close;
        if c <= 0.0 { continue; }
        if c <= running_trough {
            if let Some((trough_idx, _)) = &current_du_start {
                let duration = i - trough_idx;
                if duration > longest_du_days { longest_du_days = duration; }
            }
            current_du_start = None;
            running_trough = c;
            running_trough_idx = i;
            running_trough_date = bar.date.clone();
            if in_rally {
                let height = (c - rally_trough) / rally_trough * 100.0;
                if height >= 10.0 { rallies_10 += 1; }
                if height >= 5.0 { rallies_5 += 1; }
                in_rally = false;
            }
            rally_trough = c;
        } else {
            if current_du_start.is_none() {
                current_du_start = Some((running_trough_idx, running_trough_date.clone()));
            }
            let du = (c - running_trough) / running_trough * 100.0;
            if du > max_du_pct {
                max_du_pct = du;
                max_du_trough_date = running_trough_date.clone();
                max_du_peak_date = bar.date.clone();
            }
            if c > rally_trough {
                in_rally = true;
            }
        }
    }
    if let Some((trough_idx, _)) = &current_du_start {
        let duration = window.len().saturating_sub(*trough_idx);
        if duration > longest_du_days { longest_du_days = duration; }
    }
    if in_rally {
        let last = window.last().map(|r| r.close).unwrap_or(rally_trough);
        let height = (last - rally_trough) / rally_trough * 100.0;
        if height >= 10.0 { rallies_10 += 1; }
        if height >= 5.0 { rallies_5 += 1; }
    }
    let latest = window.last().map(|r| r.close).unwrap_or(0.0);
    let current_du = if latest > 0.0 && running_trough > 0.0 {
        (latest - running_trough) / running_trough * 100.0
    } else { 0.0 };
    let label = if max_du_pct < 5.0 {
        "MUTED"
    } else if max_du_pct < 10.0 {
        "MILD"
    } else if max_du_pct < 20.0 {
        "MEANINGFUL"
    } else if max_du_pct < 40.0 {
        "STRONG"
    } else {
        "EXPLOSIVE"
    };
    DrawupHistorySnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used,
        max_drawup_pct: max_du_pct,
        max_drawup_trough_date: max_du_trough_date,
        max_drawup_peak_date: max_du_peak_date,
        longest_drawup_days: longest_du_days,
        rallies_5pct: rallies_5,
        rallies_10pct: rallies_10,
        current_drawup_pct: current_du,
        rally_label: label.into(),
        note: String::new(),
    }
}

/// GAPSTATS compute: gap frequency and magnitude over trailing 253 sessions.
/// A "gap" is `(open_t - close_{t-1}) / close_{t-1}`. Counts only |gap| > 0.5%
/// as a real gap (avoids counting normal micro-noise).
pub fn compute_gapstats_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> GapStatsSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    if window.len() < 20 {
        return GapStatsSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            bias_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} bars", window.len()),
            ..Default::default()
        };
    }
    let mut all_gaps: Vec<f64> = Vec::new();
    let mut up_gaps: Vec<f64> = Vec::new();
    let mut down_gaps: Vec<f64> = Vec::new();
    let mut largest_up = 0.0f64;
    let mut largest_down = 0.0f64;
    for w in window.windows(2) {
        let prev_close = w[0].close;
        let curr_open = w[1].open;
        if prev_close <= 0.0 || curr_open <= 0.0 { continue; }
        let gap_pct = (curr_open - prev_close) / prev_close * 100.0;
        all_gaps.push(gap_pct);
        if gap_pct > 0.5 {
            up_gaps.push(gap_pct);
            if gap_pct > largest_up { largest_up = gap_pct; }
        } else if gap_pct < -0.5 {
            down_gaps.push(gap_pct);
            if gap_pct < largest_down { largest_down = gap_pct; }
        }
    }
    if all_gaps.is_empty() {
        return GapStatsSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            bias_label: "INSUFFICIENT_DATA".into(),
            note: "no usable open/close pairs".into(),
            ..Default::default()
        };
    }
    let avg_all: f64 = all_gaps.iter().sum::<f64>() / all_gaps.len() as f64;
    let avg_up = if up_gaps.is_empty() { 0.0 } else { up_gaps.iter().sum::<f64>() / up_gaps.len() as f64 };
    let avg_down = if down_gaps.is_empty() { 0.0 } else { down_gaps.iter().sum::<f64>() / down_gaps.len() as f64 };
    let gap_freq = ((up_gaps.len() + down_gaps.len()) as f64) / all_gaps.len() as f64 * 100.0;
    let label = if avg_all <= -0.15 {
        "DOWN_BIAS"
    } else if avg_all <= -0.05 {
        "SLIGHT_DOWN"
    } else if avg_all < 0.05 {
        "NEUTRAL"
    } else if avg_all < 0.15 {
        "SLIGHT_UP"
    } else {
        "UP_BIAS"
    };
    GapStatsSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        gap_up_count: up_gaps.len(),
        gap_down_count: down_gaps.len(),
        avg_gap_pct: avg_all,
        avg_gap_up_pct: avg_up,
        avg_gap_down_pct: avg_down,
        largest_gap_up_pct: largest_up,
        largest_gap_down_pct: largest_down,
        gap_frequency_pct: gap_freq,
        bias_label: label.into(),
        note: String::new(),
    }
}

/// VOLCLUSTER compute: ACF of r² and |r| at lags 1/5/20. Classical ARCH test.
pub fn compute_volcluster_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VolClusterSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return VolClusterSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            cluster_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} returns", log_rets.len()),
            ..Default::default()
        };
    }
    let sq: Vec<f64> = log_rets.iter().map(|r| r * r).collect();
    let abs: Vec<f64> = log_rets.iter().map(|r| r.abs()).collect();
    let sq1 = acf_at_lag(&sq, 1);
    let sq5 = acf_at_lag(&sq, 5);
    let sq20 = acf_at_lag(&sq, 20);
    let a1 = acf_at_lag(&abs, 1);
    let a5 = acf_at_lag(&abs, 5);
    let a20 = acf_at_lag(&abs, 20);
    let label = if a1 < 0.05 {
        "NONE"
    } else if a1 < 0.15 {
        "MILD"
    } else if a1 < 0.25 {
        "MODERATE"
    } else if a1 < 0.40 {
        "STRONG"
    } else {
        "VERY_STRONG"
    };
    VolClusterSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        sq_acf_lag1: sq1,
        sq_acf_lag5: sq5,
        sq_acf_lag20: sq20,
        abs_acf_lag1: a1,
        abs_acf_lag5: a5,
        abs_acf_lag20: a20,
        cluster_label: label.into(),
        note: String::new(),
    }
}

/// CLOSEPLC compute: average `(close - low) / (high - low)` placement.
pub fn compute_closeplc_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> ClosePlacementSnapshot {
    let sym = symbol.to_uppercase();
    let mut sorted: Vec<&HistoricalPriceRow> = bars.iter().collect();
    sorted.sort_by(|a, b| a.date.cmp(&b.date));
    let window: Vec<&HistoricalPriceRow> = sorted.iter().rev().take(253).rev().copied().collect();
    if window.len() < 20 {
        return ClosePlacementSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: 0,
            placement_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} bars", window.len()),
            ..Default::default()
        };
    }
    let mut positions: Vec<f64> = Vec::new();
    let mut latest_placement = 0.5;
    for bar in &window {
        if bar.high > bar.low {
            let pos = (bar.close - bar.low) / (bar.high - bar.low);
            positions.push(pos);
            latest_placement = pos;
        }
    }
    if positions.len() < 20 {
        return ClosePlacementSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: positions.len(),
            placement_label: "INSUFFICIENT_DATA".into(),
            note: "not enough non-flat bars".into(),
            ..Default::default()
        };
    }
    let avg: f64 = positions.iter().sum::<f64>() / positions.len() as f64;
    let mut sorted_pos = positions.clone();
    sorted_pos.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let median = quantile_f64(&sorted_pos, 0.5);
    let near_high = positions.iter().filter(|p| **p > 0.8).count() as f64 / positions.len() as f64 * 100.0;
    let near_low = positions.iter().filter(|p| **p < 0.2).count() as f64 / positions.len() as f64 * 100.0;
    let label = if avg < 0.3 {
        "STRONG_BEAR"
    } else if avg < 0.45 {
        "BEAR"
    } else if avg < 0.55 {
        "NEUTRAL"
    } else if avg < 0.7 {
        "BULL"
    } else {
        "STRONG_BULL"
    };
    ClosePlacementSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: positions.len(),
        avg_placement: avg,
        median_placement: median,
        latest_placement,
        pct_near_high: near_high,
        pct_near_low: near_low,
        placement_label: label.into(),
        note: String::new(),
    }
}

/// MRHL compute: AR(1) fit `r_t = α + β r_{t-1} + ε` and derive half-life.
pub fn compute_mrhl_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> MeanReversionHalfLifeSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return MeanReversionHalfLifeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} returns", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len() - 1;
    let x: Vec<f64> = log_rets[..n].to_vec();
    let y: Vec<f64> = log_rets[1..].to_vec();
    let nf = n as f64;
    let mx: f64 = x.iter().sum::<f64>() / nf;
    let my: f64 = y.iter().sum::<f64>() / nf;
    let mut sxy = 0.0f64;
    let mut sxx = 0.0f64;
    let mut syy = 0.0f64;
    for i in 0..n {
        let dx = x[i] - mx;
        let dy = y[i] - my;
        sxy += dx * dy;
        sxx += dx * dx;
        syy += dy * dy;
    }
    if sxx < f64::EPSILON {
        return MeanReversionHalfLifeSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            regime_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance in lagged series".into(),
            ..Default::default()
        };
    }
    let beta = sxy / sxx;
    let alpha = my - beta * mx;
    let r_squared = if syy > f64::EPSILON { (sxy * sxy) / (sxx * syy) } else { 0.0 };
    let (half_life, label) = if beta <= 0.0 {
        (0.0, "FAST_REVERT")
    } else if beta >= 1.0 {
        (0.0, "INSUFFICIENT_DATA")
    } else {
        let hl = -std::f64::consts::LN_2 / beta.ln();
        let lbl = if beta < 0.15 {
            "MEAN_REVERTING"
        } else if beta < 0.35 {
            "NEUTRAL"
        } else if beta < 0.60 {
            "PERSISTENT"
        } else {
            "STRONG_PERSISTENT"
        };
        (hl, lbl)
    };
    MeanReversionHalfLifeSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        beta,
        alpha,
        half_life_days: half_life,
        r_squared,
        regime_label: label.into(),
        note: String::new(),
    }
}

// ── ADR-133 Round 25 computes (DOWNVOL / SHARPR / EFFRATIO / WICKBIAS / VOLOFVOL) ──

/// DOWNVOL compute: semi-deviation + Sortino ratio.
pub fn compute_downvol_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> DownsideVolSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return DownsideVolSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            sortino_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} returns", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean: f64 = log_rets.iter().sum::<f64>() / nf;
    let mut down_sq = 0.0f64;
    let mut up_sq = 0.0f64;
    let mut total_sq = 0.0f64;
    for &r in &log_rets {
        let c = r - mean;
        total_sq += c * c;
        if r < 0.0 { down_sq += r * r; }
        if r > 0.0 { up_sq += r * r; }
    }
    let total_var = total_sq / nf;
    let down_dev = (down_sq / nf).sqrt();
    let up_dev = (up_sq / nf).sqrt();
    let sortino = if down_dev > f64::EPSILON { mean / down_dev } else { 0.0 };
    let sqrt_252 = (252.0f64).sqrt();
    let down_dev_ann = down_dev * sqrt_252;
    let sortino_ann = if down_dev_ann > f64::EPSILON { (mean * 252.0) / down_dev_ann } else { 0.0 };
    let downside_pct = if total_var > f64::EPSILON { (down_sq / nf) / total_var * 100.0 } else { 0.0 };
    let label = if down_dev < f64::EPSILON && mean <= 0.0 {
        "INSUFFICIENT_DATA"
    } else if sortino_ann < -1.0 {
        "VERY_POOR"
    } else if sortino_ann < 0.0 {
        "POOR"
    } else if sortino_ann < 1.0 {
        "NEUTRAL"
    } else if sortino_ann < 2.0 {
        "GOOD"
    } else {
        "EXCELLENT"
    };
    DownsideVolSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: n,
        mean_log_return: mean,
        downside_dev: down_dev,
        downside_dev_ann: down_dev_ann,
        upside_dev: up_dev,
        sortino_ratio: sortino,
        sortino_ratio_ann: sortino_ann,
        downside_pct_of_total: downside_pct,
        sortino_label: label.into(),
        note: String::new(),
    }
}

/// SHARPR compute: Sharpe ratio over trailing window (rf = 0).
pub fn compute_sharpr_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> SharpeRatioSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    if log_rets.len() < 30 {
        return SharpeRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            sharpe_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} returns", log_rets.len()),
            ..Default::default()
        };
    }
    let n = log_rets.len();
    let nf = n as f64;
    let mean: f64 = log_rets.iter().sum::<f64>() / nf;
    let var: f64 = log_rets.iter().map(|r| (r - mean).powi(2)).sum::<f64>() / nf;
    let stdev = var.sqrt();
    if stdev < f64::EPSILON {
        return SharpeRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: n,
            mean_log_return: mean,
            stdev_log_return: stdev,
            sharpe_label: "INSUFFICIENT_DATA".into(),
            note: "zero variance".into(),
            ..Default::default()
        };
    }
    let sharpe = mean / stdev;
    let sqrt_252 = (252.0f64).sqrt();
    let sharpe_ann = sharpe * sqrt_252;
    let mean_ann = mean * 252.0;
    let stdev_ann = stdev * sqrt_252;
    let label = if sharpe_ann < -0.5 {
        "POOR"
    } else if sharpe_ann < 0.5 {
        "BELOW_AVG"
    } else if sharpe_ann < 1.0 {
        "NEUTRAL"
    } else if sharpe_ann < 2.0 {
        "GOOD"
    } else {
        "EXCELLENT"
    };
    SharpeRatioSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: n,
        mean_log_return: mean,
        stdev_log_return: stdev,
        sharpe_ratio: sharpe,
        sharpe_ratio_ann: sharpe_ann,
        mean_return_ann: mean_ann,
        stdev_return_ann: stdev_ann,
        sharpe_label: label.into(),
        note: String::new(),
    }
}

/// EFFRATIO compute: Kaufman's efficiency ratio on closes.
pub fn compute_effratio_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> EfficiencyRatioSnapshot {
    let sym = symbol.to_uppercase();
    if bars.is_empty() {
        return EfficiencyRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            efficiency_label: "INSUFFICIENT_DATA".into(),
            note: "no bars".into(),
            ..Default::default()
        };
    }
    let n = bars.len().min(253);
    let window = &bars[bars.len().saturating_sub(n)..];
    if window.len() < 30 {
        return EfficiencyRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            efficiency_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} bars", window.len()),
            ..Default::default()
        };
    }
    let start_close = window.first().map(|b| b.close).unwrap_or(0.0);
    let end_close = window.last().map(|b| b.close).unwrap_or(0.0);
    if start_close <= 0.0 {
        return EfficiencyRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            efficiency_label: "INSUFFICIENT_DATA".into(),
            note: "start close ≤ 0".into(),
            ..Default::default()
        };
    }
    let net = end_close - start_close;
    let net_pct = (end_close / start_close - 1.0) * 100.0;
    let sum_abs: f64 = window.windows(2)
        .map(|pair| (pair[1].close - pair[0].close).abs())
        .sum();
    if sum_abs < f64::EPSILON {
        return EfficiencyRatioSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            start_close,
            end_close,
            net_change: net,
            net_change_pct: net_pct,
            sum_abs_changes: sum_abs,
            efficiency_label: "INSUFFICIENT_DATA".into(),
            note: "flat window".into(),
            ..Default::default()
        };
    }
    let er = net.abs() / sum_abs;
    let signed_er = er * net.signum();
    let label = if er < 0.10 {
        "CHOP"
    } else if er < 0.25 {
        "NOISY"
    } else if er < 0.40 {
        "MIXED"
    } else if er < 0.60 {
        "TRENDING"
    } else {
        "STRONG_TREND"
    };
    EfficiencyRatioSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: window.len(),
        start_close,
        end_close,
        net_change: net,
        net_change_pct: net_pct,
        sum_abs_changes: sum_abs,
        efficiency_ratio: er,
        signed_efficiency: signed_er,
        efficiency_label: label.into(),
        note: String::new(),
    }
}

/// WICKBIAS compute: upper vs lower wick asymmetry (requires open column).
pub fn compute_wickbias_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> WickBiasSnapshot {
    let sym = symbol.to_uppercase();
    if bars.is_empty() {
        return WickBiasSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bias_label: "INSUFFICIENT_DATA".into(),
            note: "no bars".into(),
            ..Default::default()
        };
    }
    let n = bars.len().min(253);
    let window = &bars[bars.len().saturating_sub(n)..];
    let mut uppers: Vec<f64> = Vec::with_capacity(window.len());
    let mut lowers: Vec<f64> = Vec::with_capacity(window.len());
    let mut bodies: Vec<f64> = Vec::with_capacity(window.len());
    for b in window {
        let range = b.high - b.low;
        if range <= f64::EPSILON { continue; }
        let body_top = b.open.max(b.close);
        let body_bot = b.open.min(b.close);
        let upper = (b.high - body_top) / range;
        let lower = (body_bot - b.low) / range;
        let body = (body_top - body_bot) / range;
        uppers.push(upper);
        lowers.push(lower);
        bodies.push(body);
    }
    if uppers.len() < 20 {
        return WickBiasSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: uppers.len(),
            bias_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} non-flat bars", uppers.len()),
            ..Default::default()
        };
    }
    let nf = uppers.len() as f64;
    let avg_upper: f64 = uppers.iter().sum::<f64>() / nf;
    let avg_lower: f64 = lowers.iter().sum::<f64>() / nf;
    let avg_body: f64 = bodies.iter().sum::<f64>() / nf;
    let median = |v: &mut Vec<f64>| -> f64 {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        v[v.len() / 2]
    };
    let mut up_copy = uppers.clone();
    let mut lo_copy = lowers.clone();
    let med_upper = median(&mut up_copy);
    let med_lower = median(&mut lo_copy);
    let bias = avg_lower - avg_upper;
    let label = if bias < -0.05 {
        "SELLER_REJECT"
    } else if bias < -0.02 {
        "SELLER_LEAN"
    } else if bias <= 0.02 {
        "NEUTRAL"
    } else if bias <= 0.05 {
        "BUYER_LEAN"
    } else {
        "BUYER_DEFEND"
    };
    WickBiasSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: uppers.len(),
        avg_upper_wick: avg_upper,
        avg_lower_wick: avg_lower,
        median_upper_wick: med_upper,
        median_lower_wick: med_lower,
        avg_body_share: avg_body,
        wick_bias_score: bias,
        bias_label: label.into(),
        note: String::new(),
    }
}

/// VOLOFVOL compute: stdev of rolling 20-day realized vol.
pub fn compute_volofvol_snapshot(
    symbol: &str,
    as_of: &str,
    bars: &[HistoricalPriceRow],
) -> VolOfVolSnapshot {
    let sym = symbol.to_uppercase();
    let (window, log_rets) = trailing_log_returns(bars);
    const RV_WINDOW: usize = 20;
    if log_rets.len() < RV_WINDOW + 30 {
        return VolOfVolSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: window.len(),
            cv_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} returns", log_rets.len()),
            ..Default::default()
        };
    }
    let mut rv: Vec<f64> = Vec::with_capacity(log_rets.len().saturating_sub(RV_WINDOW - 1));
    for i in (RV_WINDOW - 1)..log_rets.len() {
        let slice = &log_rets[i + 1 - RV_WINDOW..=i];
        let m: f64 = slice.iter().sum::<f64>() / (RV_WINDOW as f64);
        let v: f64 = slice.iter().map(|r| (r - m).powi(2)).sum::<f64>() / (RV_WINDOW as f64);
        rv.push(v.sqrt());
    }
    if rv.len() < 30 {
        return VolOfVolSnapshot {
            symbol: sym,
            as_of: as_of.to_string(),
            bars_used: rv.len(),
            cv_label: "INSUFFICIENT_DATA".into(),
            note: format!("only {} rv points", rv.len()),
            ..Default::default()
        };
    }
    let nf = rv.len() as f64;
    let mean_rv: f64 = rv.iter().sum::<f64>() / nf;
    let var_rv: f64 = rv.iter().map(|x| (x - mean_rv).powi(2)).sum::<f64>() / nf;
    let stdev_rv = var_rv.sqrt();
    let mut min_rv = f64::INFINITY;
    let mut max_rv = f64::NEG_INFINITY;
    for &x in &rv {
        if x < min_rv { min_rv = x; }
        if x > max_rv { max_rv = x; }
    }
    let latest_rv = *rv.last().unwrap_or(&0.0);
    let cv = if mean_rv > f64::EPSILON { stdev_rv / mean_rv } else { 0.0 };
    let label = if mean_rv < f64::EPSILON {
        "INSUFFICIENT_DATA"
    } else if cv < 0.15 {
        "STABLE"
    } else if cv < 0.25 {
        "MILD"
    } else if cv < 0.40 {
        "MODERATE"
    } else if cv < 0.60 {
        "UNSTABLE"
    } else {
        "CHAOTIC"
    };
    VolOfVolSnapshot {
        symbol: sym,
        as_of: as_of.to_string(),
        bars_used: rv.len(),
        mean_rv20: mean_rv,
        stdev_rv20: stdev_rv,
        min_rv20: min_rv,
        max_rv20: max_rv,
        latest_rv20: latest_rv,
        cv_rv20: cv,
        cv_label: label.into(),
        note: String::new(),
    }
}

// ── ADR-109 SQLite schema + helpers ────────────────────────────────────────

pub fn create_research_tables_v2(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_dividends (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_earnings_estimates (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_rating_changes (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dividends_updated ON research_dividends(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_estimates_updated ON research_earnings_estimates(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_rating_changes_updated ON research_rating_changes(updated_at);"
    ).map_err(|e| format!("create research_v2 tables: {e}"))?;
    Ok(())
}

pub fn upsert_dividends(conn: &Connection, symbol: &str, rows: &[DividendRecord]) -> Result<(), String> {
    let _ = create_research_tables_v2(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("div json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dividends(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dividends: {e}"))?;
    Ok(())
}

pub fn get_dividends(conn: &Connection, symbol: &str) -> Result<Option<Vec<DividendRecord>>, String> {
    let _ = create_research_tables_v2(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_dividends WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dividends: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_dividends: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dividends: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_earnings_estimates(conn: &Connection, symbol: &str, rows: &[EarningsEstimate]) -> Result<(), String> {
    let _ = create_research_tables_v2(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("estimates json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earnings_estimates(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert estimates: {e}"))?;
    Ok(())
}

pub fn get_earnings_estimates(conn: &Connection, symbol: &str) -> Result<Option<Vec<EarningsEstimate>>, String> {
    let _ = create_research_tables_v2(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_earnings_estimates WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_estimates: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_estimates: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_estimates: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_rating_changes(conn: &Connection, symbol: &str, rows: &[RatingChange]) -> Result<(), String> {
    let _ = create_research_tables_v2(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("rating changes json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rating_changes(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rating changes: {e}"))?;
    Ok(())
}

pub fn get_rating_changes(conn: &Connection, symbol: &str) -> Result<Option<Vec<RatingChange>>, String> {
    let _ = create_research_tables_v2(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_rating_changes WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rating_changes: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_rating_changes: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rating_changes: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── ADR-110 SQLite schema + helpers ────────────────────────────────────────

pub fn create_research_tables_v3(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_financials (
            symbol TEXT PRIMARY KEY,
            bundle_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_executives (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_financials_updated ON research_financials(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_executives_updated ON research_executives(updated_at);"
    ).map_err(|e| format!("create research_v3 tables: {e}"))?;
    Ok(())
}

pub fn upsert_financials(conn: &Connection, symbol: &str, bundle: &FinancialStatements) -> Result<(), String> {
    let _ = create_research_tables_v3(conn);
    let json = serde_json::to_string(bundle).map_err(|e| format!("financials json: {e}"))?;
    conn.execute(
        "INSERT INTO research_financials(symbol, bundle_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET bundle_json=excluded.bundle_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert financials: {e}"))?;
    Ok(())
}

pub fn get_financials(conn: &Connection, symbol: &str) -> Result<Option<FinancialStatements>, String> {
    let _ = create_research_tables_v3(conn);
    let mut stmt = conn.prepare("SELECT bundle_json FROM research_financials WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_financials: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_financials: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_financials: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_executives(conn: &Connection, symbol: &str, rows: &[Executive]) -> Result<(), String> {
    let _ = create_research_tables_v3(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("executives json: {e}"))?;
    conn.execute(
        "INSERT INTO research_executives(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert executives: {e}"))?;
    Ok(())
}

pub fn get_executives(conn: &Connection, symbol: &str) -> Result<Option<Vec<Executive>>, String> {
    let _ = create_research_tables_v3(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_executives WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_executives: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_executives: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_executives: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── ADR-111 SQLite schema + helpers ────────────────────────────────────────

pub fn create_research_tables_v4(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_stock_splits (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_etf_holdings (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_analyst_recs (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_price_target (
            symbol TEXT PRIMARY KEY,
            target_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_esg (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_index_members (
            index_code TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_stock_splits_updated ON research_stock_splits(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_etf_holdings_updated ON research_etf_holdings(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_analyst_recs_updated ON research_analyst_recs(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_price_target_updated ON research_price_target(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_esg_updated ON research_esg(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_index_members_updated ON research_index_members(updated_at);"
    ).map_err(|e| format!("create research_v4 tables: {e}"))?;
    Ok(())
}

pub fn upsert_stock_splits(conn: &Connection, symbol: &str, rows: &[StockSplit]) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("splits json: {e}"))?;
    conn.execute(
        "INSERT INTO research_stock_splits(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert stock_splits: {e}"))?;
    Ok(())
}

pub fn get_stock_splits(conn: &Connection, symbol: &str) -> Result<Option<Vec<StockSplit>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_stock_splits WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_splits: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_splits: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_splits: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_etf_holdings(conn: &Connection, symbol: &str, rows: &[EtfHolding]) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("etf holdings json: {e}"))?;
    conn.execute(
        "INSERT INTO research_etf_holdings(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert etf holdings: {e}"))?;
    Ok(())
}

pub fn get_etf_holdings(conn: &Connection, symbol: &str) -> Result<Option<Vec<EtfHolding>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_etf_holdings WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_etf_holdings: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_etf_holdings: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_etf_holdings: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_analyst_recs(conn: &Connection, symbol: &str, rows: &[AnalystRecommendation]) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("analyst recs json: {e}"))?;
    conn.execute(
        "INSERT INTO research_analyst_recs(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert analyst_recs: {e}"))?;
    Ok(())
}

pub fn get_analyst_recs(conn: &Connection, symbol: &str) -> Result<Option<Vec<AnalystRecommendation>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_analyst_recs WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_analyst_recs: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_analyst_recs: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_analyst_recs: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_price_target(conn: &Connection, symbol: &str, pt: &PriceTarget) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(pt).map_err(|e| format!("price target json: {e}"))?;
    conn.execute(
        "INSERT INTO research_price_target(symbol, target_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET target_json=excluded.target_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert price_target: {e}"))?;
    Ok(())
}

pub fn get_price_target(conn: &Connection, symbol: &str) -> Result<Option<PriceTarget>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn.prepare("SELECT target_json FROM research_price_target WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_price_target: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_price_target: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_price_target: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_esg(conn: &Connection, symbol: &str, rows: &[EsgScore]) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("esg json: {e}"))?;
    conn.execute(
        "INSERT INTO research_esg(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert esg: {e}"))?;
    Ok(())
}

pub fn get_esg(conn: &Connection, symbol: &str) -> Result<Option<Vec<EsgScore>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_esg WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_esg: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_esg: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_esg: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_index_members(conn: &Connection, index_code: &str, rows: &[IndexMember]) -> Result<(), String> {
    let _ = create_research_tables_v4(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("index members json: {e}"))?;
    conn.execute(
        "INSERT INTO research_index_members(index_code, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(index_code) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![index_code.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert index_members: {e}"))?;
    Ok(())
}

pub fn get_index_members(conn: &Connection, index_code: &str) -> Result<Option<Vec<IndexMember>>, String> {
    let _ = create_research_tables_v4(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_index_members WHERE index_code = ?1")
        .map_err(|e| format!("prepare get_index_members: {e}"))?;
    let mut r = stmt.query(params![index_code.to_uppercase()]).map_err(|e| format!("query get_index_members: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_index_members: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── ADR-112 Round 5 SQLite schema + helpers ────────────────────────────────

pub fn create_research_tables_v5(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_insider_trades (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_institutional_holders (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_shares_float (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_historical_price (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_earnings_surprise (
            symbol TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_insider_trades_updated ON research_insider_trades(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_institutional_holders_updated ON research_institutional_holders(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_shares_float_updated ON research_shares_float(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_historical_price_updated ON research_historical_price(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_earnings_surprise_updated ON research_earnings_surprise(updated_at);"
    ).map_err(|e| format!("create research_v5 tables: {e}"))?;
    Ok(())
}

pub fn upsert_insider_trades(conn: &Connection, symbol: &str, rows: &[InsiderTrade]) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("insider json: {e}"))?;
    conn.execute(
        "INSERT INTO research_insider_trades(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert insider: {e}"))?;
    Ok(())
}

pub fn get_insider_trades(conn: &Connection, symbol: &str) -> Result<Option<Vec<InsiderTrade>>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_insider_trades WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_insider: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_insider: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_insider: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_institutional_holders(conn: &Connection, symbol: &str, rows: &[InstitutionalHolder]) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("holders json: {e}"))?;
    conn.execute(
        "INSERT INTO research_institutional_holders(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert holders: {e}"))?;
    Ok(())
}

pub fn get_institutional_holders(conn: &Connection, symbol: &str) -> Result<Option<Vec<InstitutionalHolder>>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_institutional_holders WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_holders: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_holders: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_holders: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_shares_float(conn: &Connection, symbol: &str, snap: &SharesFloat) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("float json: {e}"))?;
    conn.execute(
        "INSERT INTO research_shares_float(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert float: {e}"))?;
    Ok(())
}

pub fn get_shares_float(conn: &Connection, symbol: &str) -> Result<Option<SharesFloat>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_shares_float WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_float: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_float: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_float: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_historical_price(conn: &Connection, symbol: &str, rows: &[HistoricalPriceRow]) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("hp json: {e}"))?;
    conn.execute(
        "INSERT INTO research_historical_price(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hp: {e}"))?;
    Ok(())
}

pub fn get_historical_price(conn: &Connection, symbol: &str) -> Result<Option<Vec<HistoricalPriceRow>>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_historical_price WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hp: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_hp: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hp: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_earnings_surprises(conn: &Connection, symbol: &str, rows: &[EarningsSurprise]) -> Result<(), String> {
    let _ = create_research_tables_v5(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("surprise json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earnings_surprise(symbol, rows_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert surprise: {e}"))?;
    Ok(())
}

pub fn get_earnings_surprises(conn: &Connection, symbol: &str) -> Result<Option<Vec<EarningsSurprise>>, String> {
    let _ = create_research_tables_v5(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_earnings_surprise WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_surprise: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_surprise: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_surprise: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── ADR-113 Round 6 SQLite schema + helpers ────────────────────────────────

pub fn create_research_tables_v6(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_world_indices (
            snapshot_key TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_market_movers (
            snapshot_key TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_sector_performance (
            snapshot_key TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_wacc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_world_indices_updated ON research_world_indices(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_market_movers_updated ON research_market_movers(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_sector_performance_updated ON research_sector_performance(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_wacc_updated ON research_wacc(updated_at);"
    ).map_err(|e| format!("create research_v6 tables: {e}"))?;
    Ok(())
}

pub fn upsert_world_indices(conn: &Connection, rows: &[WorldIndex]) -> Result<(), String> {
    let _ = create_research_tables_v6(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("wei json: {e}"))?;
    conn.execute(
        "INSERT INTO research_world_indices(snapshot_key, rows_json, updated_at) VALUES ('latest',?1,?2)
         ON CONFLICT(snapshot_key) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![json, now_ts()],
    ).map_err(|e| format!("upsert wei: {e}"))?;
    Ok(())
}

pub fn get_world_indices(conn: &Connection) -> Result<Option<Vec<WorldIndex>>, String> {
    let _ = create_research_tables_v6(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_world_indices WHERE snapshot_key='latest'")
        .map_err(|e| format!("prepare get_wei: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_wei: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_wei: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_market_movers(conn: &Connection, movers: &MarketMovers) -> Result<(), String> {
    let _ = create_research_tables_v6(conn);
    let json = serde_json::to_string(movers).map_err(|e| format!("mov json: {e}"))?;
    conn.execute(
        "INSERT INTO research_market_movers(snapshot_key, snapshot_json, updated_at) VALUES ('latest',?1,?2)
         ON CONFLICT(snapshot_key) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![json, now_ts()],
    ).map_err(|e| format!("upsert mov: {e}"))?;
    Ok(())
}

pub fn get_market_movers(conn: &Connection) -> Result<Option<MarketMovers>, String> {
    let _ = create_research_tables_v6(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_market_movers WHERE snapshot_key='latest'")
        .map_err(|e| format!("prepare get_mov: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_mov: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_mov: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_sector_performance(conn: &Connection, rows: &[SectorPerformance]) -> Result<(), String> {
    let _ = create_research_tables_v6(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("indu json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sector_performance(snapshot_key, rows_json, updated_at) VALUES ('latest',?1,?2)
         ON CONFLICT(snapshot_key) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![json, now_ts()],
    ).map_err(|e| format!("upsert indu: {e}"))?;
    Ok(())
}

pub fn get_sector_performance(conn: &Connection) -> Result<Option<Vec<SectorPerformance>>, String> {
    let _ = create_research_tables_v6(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_sector_performance WHERE snapshot_key='latest'")
        .map_err(|e| format!("prepare get_indu: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_indu: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_indu: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_wacc(conn: &Connection, symbol: &str, snap: &WaccSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v6(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("wacc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_wacc(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert wacc: {e}"))?;
    Ok(())
}

pub fn get_wacc(conn: &Connection, symbol: &str) -> Result<Option<WaccSnapshot>, String> {
    let _ = create_research_tables_v6(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_wacc WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_wacc: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_wacc: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_wacc: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── ADR-114 Round 7 SQLite schema + helpers ───────────────────────────────

pub fn create_research_tables_v7(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_currency_rates (
            snapshot_key TEXT PRIMARY KEY,
            rows_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_beta (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_ddm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_relative_valuation (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_figi (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_currency_rates_updated ON research_currency_rates(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_beta_updated ON research_beta(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_ddm_updated ON research_ddm(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_relative_valuation_updated ON research_relative_valuation(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_figi_updated ON research_figi(updated_at);"
    ).map_err(|e| format!("create research_v7 tables: {e}"))?;
    Ok(())
}

pub fn upsert_currency_rates(conn: &Connection, rows: &[CurrencyRate]) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(rows).map_err(|e| format!("wcr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_currency_rates(snapshot_key, rows_json, updated_at) VALUES ('latest',?1,?2)
         ON CONFLICT(snapshot_key) DO UPDATE SET rows_json=excluded.rows_json, updated_at=excluded.updated_at",
        params![json, now_ts()],
    ).map_err(|e| format!("upsert wcr: {e}"))?;
    Ok(())
}

pub fn get_currency_rates(conn: &Connection) -> Result<Option<Vec<CurrencyRate>>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn.prepare("SELECT rows_json FROM research_currency_rates WHERE snapshot_key='latest'")
        .map_err(|e| format!("prepare get_wcr: {e}"))?;
    let mut r = stmt.query([]).map_err(|e| format!("query get_wcr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_wcr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_beta(conn: &Connection, symbol: &str, snap: &BetaSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("beta json: {e}"))?;
    conn.execute(
        "INSERT INTO research_beta(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert beta: {e}"))?;
    Ok(())
}

pub fn get_beta(conn: &Connection, symbol: &str) -> Result<Option<BetaSnapshot>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_beta WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_beta: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_beta: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_beta: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_ddm(conn: &Connection, symbol: &str, snap: &DdmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ddm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ddm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ddm: {e}"))?;
    Ok(())
}

pub fn get_ddm(conn: &Connection, symbol: &str) -> Result<Option<DdmSnapshot>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_ddm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ddm: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_ddm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ddm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_relative_valuation(conn: &Connection, symbol: &str, snap: &RelativeValuation) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rv json: {e}"))?;
    conn.execute(
        "INSERT INTO research_relative_valuation(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rv: {e}"))?;
    Ok(())
}

pub fn get_relative_valuation(conn: &Connection, symbol: &str) -> Result<Option<RelativeValuation>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_relative_valuation WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rv: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_rv: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rv: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

pub fn upsert_figi(conn: &Connection, symbol: &str, snap: &FigiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v7(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("figi json: {e}"))?;
    conn.execute(
        "INSERT INTO research_figi(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert figi: {e}"))?;
    Ok(())
}

pub fn get_figi(conn: &Connection, symbol: &str) -> Result<Option<FigiSnapshot>, String> {
    let _ = create_research_tables_v7(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_figi WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_figi: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_figi: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_figi: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

// ── ADR-115 Round 8 schema: HRA / DCF / SVM / OMON / IVOL ────────────────

pub fn create_research_tables_v8(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_hra (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_dcf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_svm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_options_chain (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_ivol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hra_updated            ON research_hra(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_dcf_updated            ON research_dcf(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_svm_updated            ON research_svm(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_options_chain_updated  ON research_options_chain(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_ivol_updated           ON research_ivol(updated_at);"
    ).map_err(|e| format!("create research_v8 tables: {e}"))?;
    Ok(())
}

pub fn upsert_hra(conn: &Connection, symbol: &str, snap: &HraSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hra json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hra(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hra: {e}"))?;
    Ok(())
}

pub fn get_hra(conn: &Connection, symbol: &str) -> Result<Option<HraSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_hra WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hra: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_hra: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hra: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_dcf(conn: &Connection, symbol: &str, snap: &DcfSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dcf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dcf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dcf: {e}"))?;
    Ok(())
}

pub fn get_dcf(conn: &Connection, symbol: &str) -> Result<Option<DcfSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_dcf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dcf: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_dcf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dcf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_svm(conn: &Connection, symbol: &str, snap: &SvmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("svm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_svm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert svm: {e}"))?;
    Ok(())
}

pub fn get_svm(conn: &Connection, symbol: &str) -> Result<Option<SvmSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_svm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_svm: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_svm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_svm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_options_chain(conn: &Connection, symbol: &str, snap: &OptionsChainSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("options chain json: {e}"))?;
    conn.execute(
        "INSERT INTO research_options_chain(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert options chain: {e}"))?;
    Ok(())
}

pub fn get_options_chain(conn: &Connection, symbol: &str) -> Result<Option<OptionsChainSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_options_chain WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_options_chain: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_options_chain: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_options_chain: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_ivol(conn: &Connection, symbol: &str, snap: &IvolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v8(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ivol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ivol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ivol: {e}"))?;
    Ok(())
}

pub fn get_ivol(conn: &Connection, symbol: &str) -> Result<Option<IvolSnapshot>, String> {
    let _ = create_research_tables_v8(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_ivol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ivol: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_ivol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ivol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-116 Round 9 schema: SEAG / COR / TRA / TECH / SKEW ───────────────

pub fn create_research_tables_v9(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_seasonality (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_correlation (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_total_return (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_technicals (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_vol_skew (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_seasonality_updated  ON research_seasonality(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_correlation_updated  ON research_correlation(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_total_return_updated ON research_total_return(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_technicals_updated   ON research_technicals(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_vol_skew_updated     ON research_vol_skew(updated_at);"
    ).map_err(|e| format!("create research_v9 tables: {e}"))?;
    Ok(())
}

pub fn upsert_seasonality(conn: &Connection, symbol: &str, snap: &SeasonalitySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("seasonality json: {e}"))?;
    conn.execute(
        "INSERT INTO research_seasonality(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert seasonality: {e}"))?;
    Ok(())
}

pub fn get_seasonality(conn: &Connection, symbol: &str) -> Result<Option<SeasonalitySnapshot>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_seasonality WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_seasonality: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_seasonality: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_seasonality: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_correlation(conn: &Connection, symbol: &str, snap: &CorrelationMatrix) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("correlation json: {e}"))?;
    conn.execute(
        "INSERT INTO research_correlation(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert correlation: {e}"))?;
    Ok(())
}

pub fn get_correlation(conn: &Connection, symbol: &str) -> Result<Option<CorrelationMatrix>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_correlation WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_correlation: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_correlation: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_correlation: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_total_return(conn: &Connection, symbol: &str, snap: &TotalReturnSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("total return json: {e}"))?;
    conn.execute(
        "INSERT INTO research_total_return(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert total return: {e}"))?;
    Ok(())
}

pub fn get_total_return(conn: &Connection, symbol: &str) -> Result<Option<TotalReturnSnapshot>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_total_return WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_total_return: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_total_return: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_total_return: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_technicals(conn: &Connection, symbol: &str, snap: &TechnicalSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("technicals json: {e}"))?;
    conn.execute(
        "INSERT INTO research_technicals(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert technicals: {e}"))?;
    Ok(())
}

pub fn get_technicals(conn: &Connection, symbol: &str) -> Result<Option<TechnicalSnapshot>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_technicals WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_technicals: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_technicals: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_technicals: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_vol_skew(conn: &Connection, symbol: &str, snap: &VolatilitySkew) -> Result<(), String> {
    let _ = create_research_tables_v9(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vol skew json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vol_skew(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert vol skew: {e}"))?;
    Ok(())
}

pub fn get_vol_skew(conn: &Connection, symbol: &str) -> Result<Option<VolatilitySkew>, String> {
    let _ = create_research_tables_v9(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_vol_skew WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_vol_skew: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_vol_skew: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_vol_skew: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-117 Round 10 schema: LEV / ACRL / RVOL / FCFY / SHRT ──────────────

pub fn create_research_tables_v10(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_leverage (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_accruals (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_realized_vol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_fcf_yield (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_short_interest (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_leverage_updated        ON research_leverage(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_accruals_updated        ON research_accruals(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_realized_vol_updated    ON research_realized_vol(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_fcf_yield_updated       ON research_fcf_yield(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_short_interest_updated  ON research_short_interest(updated_at);"
    ).map_err(|e| format!("create research_v10 tables: {e}"))?;
    Ok(())
}

pub fn upsert_leverage(conn: &Connection, symbol: &str, snap: &LeverageSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("leverage json: {e}"))?;
    conn.execute(
        "INSERT INTO research_leverage(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert leverage: {e}"))?;
    Ok(())
}

pub fn get_leverage(conn: &Connection, symbol: &str) -> Result<Option<LeverageSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_leverage WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_leverage: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_leverage: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_leverage: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_accruals(conn: &Connection, symbol: &str, snap: &AccrualsSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("accruals json: {e}"))?;
    conn.execute(
        "INSERT INTO research_accruals(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert accruals: {e}"))?;
    Ok(())
}

pub fn get_accruals(conn: &Connection, symbol: &str) -> Result<Option<AccrualsSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_accruals WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_accruals: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_accruals: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_accruals: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_realized_vol(conn: &Connection, symbol: &str, snap: &RealizedVolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("realized vol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_realized_vol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert realized vol: {e}"))?;
    Ok(())
}

pub fn get_realized_vol(conn: &Connection, symbol: &str) -> Result<Option<RealizedVolSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_realized_vol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_realized_vol: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_realized_vol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_realized_vol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_fcf_yield(conn: &Connection, symbol: &str, snap: &FcfYieldSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fcf yield json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fcf_yield(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fcf yield: {e}"))?;
    Ok(())
}

pub fn get_fcf_yield(conn: &Connection, symbol: &str) -> Result<Option<FcfYieldSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_fcf_yield WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fcf_yield: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_fcf_yield: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fcf_yield: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_short_interest(conn: &Connection, symbol: &str, snap: &ShortInterestSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v10(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("short interest json: {e}"))?;
    conn.execute(
        "INSERT INTO research_short_interest(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert short interest: {e}"))?;
    Ok(())
}

pub fn get_short_interest(conn: &Connection, symbol: &str) -> Result<Option<ShortInterestSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_short_interest WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_short_interest: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_short_interest: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_short_interest: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-118 Godel Parity Round 11 schema + helpers ─────────────────────────

pub fn create_research_tables_v11(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_altman_z (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_piotroski (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_ohlc_vol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_eps_beat (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_price_target_dispersion (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_altman_z_updated                 ON research_altman_z(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_piotroski_updated                ON research_piotroski(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_ohlc_vol_updated                 ON research_ohlc_vol(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_eps_beat_updated                 ON research_eps_beat(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_price_target_dispersion_updated  ON research_price_target_dispersion(updated_at);"
    ).map_err(|e| format!("create research_v11 tables: {e}"))?;
    Ok(())
}

pub fn upsert_altman_z(conn: &Connection, symbol: &str, snap: &AltmanZSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("altman_z json: {e}"))?;
    conn.execute(
        "INSERT INTO research_altman_z(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert altman_z: {e}"))?;
    Ok(())
}

pub fn get_altman_z(conn: &Connection, symbol: &str) -> Result<Option<AltmanZSnapshot>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_altman_z WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_altman_z: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_altman_z: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_altman_z: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_piotroski(conn: &Connection, symbol: &str, snap: &PiotroskiSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("piotroski json: {e}"))?;
    conn.execute(
        "INSERT INTO research_piotroski(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert piotroski: {e}"))?;
    Ok(())
}

pub fn get_piotroski(conn: &Connection, symbol: &str) -> Result<Option<PiotroskiSnapshot>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_piotroski WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_piotroski: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_piotroski: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_piotroski: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_ohlc_vol(conn: &Connection, symbol: &str, snap: &OhlcVolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ohlc_vol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ohlc_vol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ohlc_vol: {e}"))?;
    Ok(())
}

pub fn get_ohlc_vol(conn: &Connection, symbol: &str) -> Result<Option<OhlcVolSnapshot>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_ohlc_vol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ohlc_vol: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_ohlc_vol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ohlc_vol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_eps_beat(conn: &Connection, symbol: &str, snap: &EpsBeatSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("eps_beat json: {e}"))?;
    conn.execute(
        "INSERT INTO research_eps_beat(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert eps_beat: {e}"))?;
    Ok(())
}

pub fn get_eps_beat(conn: &Connection, symbol: &str) -> Result<Option<EpsBeatSnapshot>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_eps_beat WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_eps_beat: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_eps_beat: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_eps_beat: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_price_target_dispersion(conn: &Connection, symbol: &str, snap: &PriceTargetDispersion) -> Result<(), String> {
    let _ = create_research_tables_v11(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("price_target_dispersion json: {e}"))?;
    conn.execute(
        "INSERT INTO research_price_target_dispersion(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert price_target_dispersion: {e}"))?;
    Ok(())
}

pub fn get_price_target_dispersion(conn: &Connection, symbol: &str) -> Result<Option<PriceTargetDispersion>, String> {
    let _ = create_research_tables_v11(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_price_target_dispersion WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_price_target_dispersion: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_price_target_dispersion: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_price_target_dispersion: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-119 Godel Parity Round 12 schema + helpers ─────────────────────────

pub fn create_research_tables_v12(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_insider_activity (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_divg (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_earm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_sector_rotation (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_updm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_insider_activity_updated ON research_insider_activity(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_divg_updated             ON research_divg(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_earm_updated             ON research_earm(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_sector_rotation_updated  ON research_sector_rotation(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_updm_updated             ON research_updm(updated_at);"
    ).map_err(|e| format!("create research_v12 tables: {e}"))?;
    Ok(())
}

pub fn upsert_insider_activity(conn: &Connection, symbol: &str, snap: &InsiderActivitySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("insider_activity json: {e}"))?;
    conn.execute(
        "INSERT INTO research_insider_activity(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert insider_activity: {e}"))?;
    Ok(())
}

pub fn get_insider_activity(conn: &Connection, symbol: &str) -> Result<Option<InsiderActivitySnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_insider_activity WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_insider_activity: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_insider_activity: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_insider_activity: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_divg(conn: &Connection, symbol: &str, snap: &DivgSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("divg json: {e}"))?;
    conn.execute(
        "INSERT INTO research_divg(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert divg: {e}"))?;
    Ok(())
}

pub fn get_divg(conn: &Connection, symbol: &str) -> Result<Option<DivgSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_divg WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_divg: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_divg: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_divg: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_earm(conn: &Connection, symbol: &str, snap: &EarmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("earm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert earm: {e}"))?;
    Ok(())
}

pub fn get_earm(conn: &Connection, symbol: &str) -> Result<Option<EarmSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_earm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_earm: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_earm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_earm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_sector_rotation(conn: &Connection, symbol: &str, snap: &SectorRotationSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sector_rotation json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sector_rotation(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sector_rotation: {e}"))?;
    Ok(())
}

pub fn get_sector_rotation(conn: &Connection, symbol: &str) -> Result<Option<SectorRotationSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_sector_rotation WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sector_rotation: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_sector_rotation: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sector_rotation: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_updm(conn: &Connection, symbol: &str, snap: &UpdmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v12(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("updm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_updm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert updm: {e}"))?;
    Ok(())
}

pub fn get_updm(conn: &Connection, symbol: &str) -> Result<Option<UpdmSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_updm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_updm: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_updm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_updm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-120 Godel Parity Round 13 schema + helpers ─────────────────────────

pub fn create_research_tables_v13(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_momentum (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_momentum_updated ON research_momentum(updated_at);

        CREATE TABLE IF NOT EXISTS research_liquidity (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_liquidity_updated ON research_liquidity(updated_at);

        CREATE TABLE IF NOT EXISTS research_breakout (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_breakout_updated ON research_breakout(updated_at);

        CREATE TABLE IF NOT EXISTS research_cash_cycle (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_cash_cycle_updated ON research_cash_cycle(updated_at);

        CREATE TABLE IF NOT EXISTS research_credit (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_credit_updated ON research_credit(updated_at);",
    ).map_err(|e| format!("create v13 tables: {e}"))?;
    Ok(())
}

pub fn upsert_momentum(conn: &Connection, symbol: &str, snap: &MomentumSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("momentum json: {e}"))?;
    conn.execute(
        "INSERT INTO research_momentum(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert momentum: {e}"))?;
    Ok(())
}

pub fn get_momentum(conn: &Connection, symbol: &str) -> Result<Option<MomentumSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_momentum WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_momentum: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_momentum: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_momentum: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_liquidity(conn: &Connection, symbol: &str, snap: &LiquiditySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("liquidity json: {e}"))?;
    conn.execute(
        "INSERT INTO research_liquidity(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert liquidity: {e}"))?;
    Ok(())
}

pub fn get_liquidity(conn: &Connection, symbol: &str) -> Result<Option<LiquiditySnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_liquidity WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_liquidity: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_liquidity: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_liquidity: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_breakout(conn: &Connection, symbol: &str, snap: &BreakoutSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("breakout json: {e}"))?;
    conn.execute(
        "INSERT INTO research_breakout(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert breakout: {e}"))?;
    Ok(())
}

pub fn get_breakout(conn: &Connection, symbol: &str) -> Result<Option<BreakoutSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_breakout WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_breakout: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_breakout: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_breakout: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_cash_cycle(conn: &Connection, symbol: &str, snap: &CashCycleSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("cash_cycle json: {e}"))?;
    conn.execute(
        "INSERT INTO research_cash_cycle(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert cash_cycle: {e}"))?;
    Ok(())
}

pub fn get_cash_cycle(conn: &Connection, symbol: &str) -> Result<Option<CashCycleSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_cash_cycle WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_cash_cycle: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_cash_cycle: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_cash_cycle: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_credit(conn: &Connection, symbol: &str, snap: &CreditSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v13(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("credit json: {e}"))?;
    conn.execute(
        "INSERT INTO research_credit(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert credit: {e}"))?;
    Ok(())
}

pub fn get_credit(conn: &Connection, symbol: &str) -> Result<Option<CreditSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_credit WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_credit: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_credit: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_credit: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-121 Godel Parity Round 14 schema + helpers ─────────────────────────

pub fn create_research_tables_v14(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_growm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_growm_updated ON research_growm(updated_at);

        CREATE TABLE IF NOT EXISTS research_flow (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_flow_updated ON research_flow(updated_at);

        CREATE TABLE IF NOT EXISTS research_regime (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_regime_updated ON research_regime(updated_at);

        CREATE TABLE IF NOT EXISTS research_relvol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_relvol_updated ON research_relvol(updated_at);

        CREATE TABLE IF NOT EXISTS research_margins (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_margins_updated ON research_margins(updated_at);",
    ).map_err(|e| format!("create v14 tables: {e}"))?;
    Ok(())
}

pub fn upsert_growm(conn: &Connection, symbol: &str, snap: &GrowmSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("growm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_growm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert growm: {e}"))?;
    Ok(())
}

pub fn get_growm(conn: &Connection, symbol: &str) -> Result<Option<GrowmSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_growm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_growm: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_growm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_growm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_flow(conn: &Connection, symbol: &str, snap: &FlowSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("flow json: {e}"))?;
    conn.execute(
        "INSERT INTO research_flow(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert flow: {e}"))?;
    Ok(())
}

pub fn get_flow(conn: &Connection, symbol: &str) -> Result<Option<FlowSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_flow WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_flow: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_flow: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_flow: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_regime(conn: &Connection, symbol: &str, snap: &RegimeSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("regime json: {e}"))?;
    conn.execute(
        "INSERT INTO research_regime(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert regime: {e}"))?;
    Ok(())
}

pub fn get_regime(conn: &Connection, symbol: &str) -> Result<Option<RegimeSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_regime WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_regime: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_regime: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_regime: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_relvol(conn: &Connection, symbol: &str, snap: &RelVolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("relvol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_relvol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert relvol: {e}"))?;
    Ok(())
}

pub fn get_relvol(conn: &Connection, symbol: &str) -> Result<Option<RelVolSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_relvol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_relvol: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_relvol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_relvol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_margins(conn: &Connection, symbol: &str, snap: &MarginsSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v14(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("margins json: {e}"))?;
    conn.execute(
        "INSERT INTO research_margins(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert margins: {e}"))?;
    Ok(())
}

pub fn get_margins(conn: &Connection, symbol: &str) -> Result<Option<MarginsSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_margins WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_margins: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_margins: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_margins: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-122 Godel Parity Round 15 schema + helpers ─────────────────────────

pub fn create_research_tables_v15(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_val (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_val_updated ON research_val(updated_at);

        CREATE TABLE IF NOT EXISTS research_qual (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_qual_updated ON research_qual(updated_at);

        CREATE TABLE IF NOT EXISTS research_risk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_risk_updated ON research_risk(updated_at);

        CREATE TABLE IF NOT EXISTS research_insstrk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_insstrk_updated ON research_insstrk(updated_at);

        CREATE TABLE IF NOT EXISTS research_covg (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_covg_updated ON research_covg(updated_at);",
    ).map_err(|e| format!("create v15 tables: {e}"))?;
    Ok(())
}

pub fn upsert_val(conn: &Connection, symbol: &str, snap: &ValueSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("val json: {e}"))?;
    conn.execute(
        "INSERT INTO research_val(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert val: {e}"))?;
    Ok(())
}

pub fn get_val(conn: &Connection, symbol: &str) -> Result<Option<ValueSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_val WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_val: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_val: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_val: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_qual(conn: &Connection, symbol: &str, snap: &QualitySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("qual json: {e}"))?;
    conn.execute(
        "INSERT INTO research_qual(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert qual: {e}"))?;
    Ok(())
}

pub fn get_qual(conn: &Connection, symbol: &str) -> Result<Option<QualitySnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_qual WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_qual: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_qual: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_qual: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_risk(conn: &Connection, symbol: &str, snap: &RiskSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("risk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_risk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert risk: {e}"))?;
    Ok(())
}

pub fn get_risk(conn: &Connection, symbol: &str) -> Result<Option<RiskSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_risk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_risk: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_risk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_risk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_insstrk(conn: &Connection, symbol: &str, snap: &InsiderStreakSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("insstrk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_insstrk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert insstrk: {e}"))?;
    Ok(())
}

pub fn get_insstrk(conn: &Connection, symbol: &str) -> Result<Option<InsiderStreakSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_insstrk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_insstrk: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_insstrk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_insstrk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_covg(conn: &Connection, symbol: &str, snap: &CoverageSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("covg json: {e}"))?;
    conn.execute(
        "INSERT INTO research_covg(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert covg: {e}"))?;
    Ok(())
}

pub fn get_covg(conn: &Connection, symbol: &str) -> Result<Option<CoverageSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_covg WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_covg: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_covg: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_covg: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-123 Round 16 schema + helpers ──────────────────────────────────────

pub fn create_research_tables_v16(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v15(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_vrk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_vrk_updated ON research_vrk(updated_at);

        CREATE TABLE IF NOT EXISTS research_qrk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_qrk_updated ON research_qrk(updated_at);

        CREATE TABLE IF NOT EXISTS research_rrk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rrk_updated ON research_rrk(updated_at);

        CREATE TABLE IF NOT EXISTS research_relepsgr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_relepsgr_updated ON research_relepsgr(updated_at);

        CREATE TABLE IF NOT EXISTS research_pead (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pead_updated ON research_pead(updated_at);",
    ).map_err(|e| format!("create v16 tables: {e}"))?;
    Ok(())
}

pub fn upsert_vrk(conn: &Connection, symbol: &str, snap: &ValueRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("vrk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_vrk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert vrk: {e}"))?;
    Ok(())
}

pub fn get_vrk(conn: &Connection, symbol: &str) -> Result<Option<ValueRankSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_vrk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_vrk: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_vrk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_vrk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_qrk(conn: &Connection, symbol: &str, snap: &QualityRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("qrk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_qrk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert qrk: {e}"))?;
    Ok(())
}

pub fn get_qrk(conn: &Connection, symbol: &str) -> Result<Option<QualityRankSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_qrk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_qrk: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_qrk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_qrk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_rrk(conn: &Connection, symbol: &str, snap: &RiskRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rrk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rrk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rrk: {e}"))?;
    Ok(())
}

pub fn get_rrk(conn: &Connection, symbol: &str) -> Result<Option<RiskRankSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_rrk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rrk: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_rrk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rrk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_relepsgr(conn: &Connection, symbol: &str, snap: &RelativeEpsGrowthSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("relepsgr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_relepsgr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert relepsgr: {e}"))?;
    Ok(())
}

pub fn get_relepsgr(conn: &Connection, symbol: &str) -> Result<Option<RelativeEpsGrowthSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_relepsgr WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_relepsgr: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_relepsgr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_relepsgr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_pead(conn: &Connection, symbol: &str, snap: &PeadSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pead json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pead(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pead: {e}"))?;
    Ok(())
}

pub fn get_pead(conn: &Connection, symbol: &str) -> Result<Option<PeadSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_pead WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pead: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_pead: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pead: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

/// Whole-table scan of `research_val`. Used by VRK / sector-rank surfaces.
pub fn get_all_val(conn: &Connection) -> Result<Vec<ValueSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_val")
        .map_err(|e| format!("prepare get_all_val: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_val: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<ValueSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_qual`. Used by QRK.
pub fn get_all_qual(conn: &Connection) -> Result<Vec<QualitySnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_qual")
        .map_err(|e| format!("prepare get_all_qual: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_qual: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<QualitySnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_risk`. Used by RRK.
pub fn get_all_risk(conn: &Connection) -> Result<Vec<RiskSnapshot>, String> {
    let _ = create_research_tables_v15(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_risk")
        .map_err(|e| format!("prepare get_all_risk: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_risk: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<RiskSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

// ── ADR-124 Round 17 schema + wrappers ────────────────────────────────────

pub fn create_research_tables_v17(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v16(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_sizef (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_sizef_updated ON research_sizef(updated_at);

        CREATE TABLE IF NOT EXISTS research_momf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_momf_updated ON research_momf(updated_at);

        CREATE TABLE IF NOT EXISTS research_peadrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_peadrank_updated ON research_peadrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_fqm (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_fqm_updated ON research_fqm(updated_at);

        CREATE TABLE IF NOT EXISTS research_revrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_revrank_updated ON research_revrank(updated_at);",
    ).map_err(|e| format!("create v17 tables: {e}"))?;
    Ok(())
}

pub fn upsert_sizef(conn: &Connection, symbol: &str, snap: &SizeFactorSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sizef json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sizef(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sizef: {e}"))?;
    Ok(())
}

pub fn get_sizef(conn: &Connection, symbol: &str) -> Result<Option<SizeFactorSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_sizef WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sizef: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_sizef: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sizef: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_momf(conn: &Connection, symbol: &str, snap: &MomentumRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("momf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_momf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert momf: {e}"))?;
    Ok(())
}

pub fn get_momf(conn: &Connection, symbol: &str) -> Result<Option<MomentumRankSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_momf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_momf: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_momf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_momf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_peadrank(conn: &Connection, symbol: &str, snap: &PeadRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("peadrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_peadrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert peadrank: {e}"))?;
    Ok(())
}

pub fn get_peadrank(conn: &Connection, symbol: &str) -> Result<Option<PeadRankSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_peadrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_peadrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_peadrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_peadrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_fqm(conn: &Connection, symbol: &str, snap: &FundamentalQualityMeterSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fqm json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fqm(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fqm: {e}"))?;
    Ok(())
}

pub fn get_fqm(conn: &Connection, symbol: &str) -> Result<Option<FundamentalQualityMeterSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_fqm WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fqm: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_fqm: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fqm: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_revrank(conn: &Connection, symbol: &str, snap: &RevenueGrowthRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("revrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_revrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert revrank: {e}"))?;
    Ok(())
}

pub fn get_revrank(conn: &Connection, symbol: &str) -> Result<Option<RevenueGrowthRankSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_revrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_revrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_revrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_revrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

/// Whole-table scan of `research_momentum`. Used by MOMF.
pub fn get_all_momentum(conn: &Connection) -> Result<Vec<MomentumSnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_momentum")
        .map_err(|e| format!("prepare get_all_momentum: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_momentum: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<MomentumSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_pead`. Used by PEADRANK.
pub fn get_all_pead(conn: &Connection) -> Result<Vec<PeadSnapshot>, String> {
    let _ = create_research_tables_v16(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_pead")
        .map_err(|e| format!("prepare get_all_pead: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_pead: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<PeadSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

// ── ADR-125 Round 18 schema + wrappers ────────────────────────────────────

pub fn create_research_tables_v18(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v17(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_levrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_levrank_updated ON research_levrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_operank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_operank_updated ON research_operank(updated_at);

        CREATE TABLE IF NOT EXISTS research_fqmrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_fqmrank_updated ON research_fqmrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_liqrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_liqrank_updated ON research_liqrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_surpstk (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_surpstk_updated ON research_surpstk(updated_at);",
    ).map_err(|e| format!("create v18 tables: {e}"))?;
    Ok(())
}

pub fn upsert_levrank(conn: &Connection, symbol: &str, snap: &LeverageRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("levrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_levrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert levrank: {e}"))?;
    Ok(())
}

pub fn get_levrank(conn: &Connection, symbol: &str) -> Result<Option<LeverageRankSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_levrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_levrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_levrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_levrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_operank(conn: &Connection, symbol: &str, snap: &OperatingQualityRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("operank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_operank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert operank: {e}"))?;
    Ok(())
}

pub fn get_operank(conn: &Connection, symbol: &str) -> Result<Option<OperatingQualityRankSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_operank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_operank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_operank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_operank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_fqmrank(conn: &Connection, symbol: &str, snap: &FqmRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fqmrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fqmrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fqmrank: {e}"))?;
    Ok(())
}

pub fn get_fqmrank(conn: &Connection, symbol: &str) -> Result<Option<FqmRankSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_fqmrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fqmrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_fqmrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fqmrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_liqrank(conn: &Connection, symbol: &str, snap: &LiquidityRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("liqrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_liqrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert liqrank: {e}"))?;
    Ok(())
}

pub fn get_liqrank(conn: &Connection, symbol: &str) -> Result<Option<LiquidityRankSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_liqrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_liqrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_liqrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_liqrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_surpstk(conn: &Connection, symbol: &str, snap: &EarningsSurpriseStreakSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("surpstk json: {e}"))?;
    conn.execute(
        "INSERT INTO research_surpstk(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert surpstk: {e}"))?;
    Ok(())
}

pub fn get_surpstk(conn: &Connection, symbol: &str) -> Result<Option<EarningsSurpriseStreakSnapshot>, String> {
    let _ = create_research_tables_v18(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_surpstk WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_surpstk: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_surpstk: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_surpstk: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

/// Whole-table scan of `research_leverage`. Used by LEVRANK.
pub fn get_all_leverage(conn: &Connection) -> Result<Vec<LeverageSnapshot>, String> {
    let _ = create_research_tables_v10(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_leverage")
        .map_err(|e| format!("prepare get_all_leverage: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_leverage: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<LeverageSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_margins`. Used by OPERANK.
pub fn get_all_margins(conn: &Connection) -> Result<Vec<MarginsSnapshot>, String> {
    let _ = create_research_tables_v14(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_margins")
        .map_err(|e| format!("prepare get_all_margins: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_margins: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<MarginsSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_fqm`. Used by FQMRANK.
pub fn get_all_fqm(conn: &Connection) -> Result<Vec<FundamentalQualityMeterSnapshot>, String> {
    let _ = create_research_tables_v17(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_fqm")
        .map_err(|e| format!("prepare get_all_fqm: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_fqm: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<FundamentalQualityMeterSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_liquidity`. Used by LIQRANK.
pub fn get_all_liquidity(conn: &Connection) -> Result<Vec<LiquiditySnapshot>, String> {
    let _ = create_research_tables_v13(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_liquidity")
        .map_err(|e| format!("prepare get_all_liquidity: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_liquidity: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<LiquiditySnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

// ── ADR-126 Round 19 schema + wrappers ────────────────────────────────────

pub fn create_research_tables_v19(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v18(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_dvdrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dvdrank_updated ON research_dvdrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_earmrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_earmrank_updated ON research_earmrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_updgrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_updgrank_updated ON research_updgrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_gy (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_gy_updated ON research_gy(updated_at);

        CREATE TABLE IF NOT EXISTS research_des (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_des_updated ON research_des(updated_at);",
    ).map_err(|e| format!("create v19 tables: {e}"))?;
    Ok(())
}

pub fn upsert_dvdrank(conn: &Connection, symbol: &str, snap: &DividendGrowthRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dvdrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dvdrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dvdrank: {e}"))?;
    Ok(())
}

pub fn get_dvdrank(conn: &Connection, symbol: &str) -> Result<Option<DividendGrowthRankSnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_dvdrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dvdrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_dvdrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dvdrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_earmrank(conn: &Connection, symbol: &str, snap: &EarningsMomentumRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("earmrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_earmrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert earmrank: {e}"))?;
    Ok(())
}

pub fn get_earmrank(conn: &Connection, symbol: &str) -> Result<Option<EarningsMomentumRankSnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_earmrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_earmrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_earmrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_earmrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_updgrank(conn: &Connection, symbol: &str, snap: &UpgradeDowngradeRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("updgrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_updgrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert updgrank: {e}"))?;
    Ok(())
}

pub fn get_updgrank(conn: &Connection, symbol: &str) -> Result<Option<UpgradeDowngradeRankSnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_updgrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_updgrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_updgrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_updgrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_gy(conn: &Connection, symbol: &str, snap: &GapYearlySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("gy json: {e}"))?;
    conn.execute(
        "INSERT INTO research_gy(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert gy: {e}"))?;
    Ok(())
}

pub fn get_gy(conn: &Connection, symbol: &str) -> Result<Option<GapYearlySnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_gy WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_gy: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_gy: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_gy: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_des(conn: &Connection, symbol: &str, snap: &DailyEventStreakSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("des json: {e}"))?;
    conn.execute(
        "INSERT INTO research_des(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert des: {e}"))?;
    Ok(())
}

pub fn get_des(conn: &Connection, symbol: &str) -> Result<Option<DailyEventStreakSnapshot>, String> {
    let _ = create_research_tables_v19(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_des WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_des: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_des: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_des: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-127 Round 20 schema + wrappers ────────────────────────────────────

pub fn create_research_tables_v20(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v19(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_dvdyieldrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dvdyieldrank_updated ON research_dvdyieldrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_shrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_shrank_updated ON research_shrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_atrann (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_atrann_updated ON research_atrann(updated_at);

        CREATE TABLE IF NOT EXISTS research_ddhist (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_ddhist_updated ON research_ddhist(updated_at);

        CREATE TABLE IF NOT EXISTS research_priceperf (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_priceperf_updated ON research_priceperf(updated_at);",
    ).map_err(|e| format!("create v20 tables: {e}"))?;
    Ok(())
}

pub fn upsert_dvdyieldrank(conn: &Connection, symbol: &str, snap: &DividendYieldRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dvdyieldrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dvdyieldrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dvdyieldrank: {e}"))?;
    Ok(())
}

pub fn get_dvdyieldrank(conn: &Connection, symbol: &str) -> Result<Option<DividendYieldRankSnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_dvdyieldrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dvdyieldrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_dvdyieldrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dvdyieldrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_shrank(conn: &Connection, symbol: &str, snap: &ShortInterestRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("shrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_shrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert shrank: {e}"))?;
    Ok(())
}

pub fn get_shrank(conn: &Connection, symbol: &str) -> Result<Option<ShortInterestRankSnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_shrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_shrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_shrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_shrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_atrann(conn: &Connection, symbol: &str, snap: &AnnualizedAtrSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("atrann json: {e}"))?;
    conn.execute(
        "INSERT INTO research_atrann(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert atrann: {e}"))?;
    Ok(())
}

pub fn get_atrann(conn: &Connection, symbol: &str) -> Result<Option<AnnualizedAtrSnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_atrann WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_atrann: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_atrann: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_atrann: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_ddhist(conn: &Connection, symbol: &str, snap: &DrawdownHistorySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ddhist json: {e}"))?;
    conn.execute(
        "INSERT INTO research_ddhist(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ddhist: {e}"))?;
    Ok(())
}

pub fn get_ddhist(conn: &Connection, symbol: &str) -> Result<Option<DrawdownHistorySnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_ddhist WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ddhist: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_ddhist: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ddhist: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_priceperf(conn: &Connection, symbol: &str, snap: &PricePerformanceSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("priceperf json: {e}"))?;
    conn.execute(
        "INSERT INTO research_priceperf(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert priceperf: {e}"))?;
    Ok(())
}

pub fn get_priceperf(conn: &Connection, symbol: &str) -> Result<Option<PricePerformanceSnapshot>, String> {
    let _ = create_research_tables_v20(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_priceperf WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_priceperf: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_priceperf: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_priceperf: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-128 Round 21 schema v21 + wrappers ──

pub fn create_research_tables_v21(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v20(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_betarank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_betarank_updated ON research_betarank(updated_at);

        CREATE TABLE IF NOT EXISTS research_pegrank (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_pegrank_updated ON research_pegrank(updated_at);

        CREATE TABLE IF NOT EXISTS research_fhighlow (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_fhighlow_updated ON research_fhighlow(updated_at);

        CREATE TABLE IF NOT EXISTS research_rvcone (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_rvcone_updated ON research_rvcone(updated_at);

        CREATE TABLE IF NOT EXISTS research_calpb (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_calpb_updated ON research_calpb(updated_at);",
    ).map_err(|e| format!("create v21 tables: {e}"))?;
    Ok(())
}

pub fn upsert_betarank(conn: &Connection, symbol: &str, snap: &BetaRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("betarank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_betarank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert betarank: {e}"))?;
    Ok(())
}

pub fn get_betarank(conn: &Connection, symbol: &str) -> Result<Option<BetaRankSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_betarank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_betarank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_betarank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_betarank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_pegrank(conn: &Connection, symbol: &str, snap: &PegRankSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("pegrank json: {e}"))?;
    conn.execute(
        "INSERT INTO research_pegrank(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert pegrank: {e}"))?;
    Ok(())
}

pub fn get_pegrank(conn: &Connection, symbol: &str) -> Result<Option<PegRankSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_pegrank WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_pegrank: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_pegrank: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_pegrank: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_fhighlow(conn: &Connection, symbol: &str, snap: &FiftyTwoWeekHighLowSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("fhighlow json: {e}"))?;
    conn.execute(
        "INSERT INTO research_fhighlow(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert fhighlow: {e}"))?;
    Ok(())
}

pub fn get_fhighlow(conn: &Connection, symbol: &str) -> Result<Option<FiftyTwoWeekHighLowSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_fhighlow WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_fhighlow: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_fhighlow: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_fhighlow: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_rvcone(conn: &Connection, symbol: &str, snap: &RealizedVolConeSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("rvcone json: {e}"))?;
    conn.execute(
        "INSERT INTO research_rvcone(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert rvcone: {e}"))?;
    Ok(())
}

pub fn get_rvcone(conn: &Connection, symbol: &str) -> Result<Option<RealizedVolConeSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_rvcone WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_rvcone: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_rvcone: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_rvcone: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_calpb(conn: &Connection, symbol: &str, snap: &CalendarPeriodBreakdownSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("calpb json: {e}"))?;
    conn.execute(
        "INSERT INTO research_calpb(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert calpb: {e}"))?;
    Ok(())
}

pub fn get_calpb(conn: &Connection, symbol: &str) -> Result<Option<CalendarPeriodBreakdownSnapshot>, String> {
    let _ = create_research_tables_v21(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_calpb WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_calpb: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_calpb: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_calpb: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-129 Round 22 schema v22 + wrappers ──

pub fn create_research_tables_v22(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v21(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_retskew (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_retskew_updated ON research_retskew(updated_at);

        CREATE TABLE IF NOT EXISTS research_retkurt (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_retkurt_updated ON research_retkurt(updated_at);

        CREATE TABLE IF NOT EXISTS research_tailr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_tailr_updated ON research_tailr(updated_at);

        CREATE TABLE IF NOT EXISTS research_runlen (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_runlen_updated ON research_runlen(updated_at);

        CREATE TABLE IF NOT EXISTS research_dayrange (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_dayrange_updated ON research_dayrange(updated_at);",
    ).map_err(|e| format!("create v22 tables: {e}"))?;
    Ok(())
}

pub fn upsert_retskew(conn: &Connection, symbol: &str, snap: &ReturnSkewnessSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("retskew json: {e}"))?;
    conn.execute(
        "INSERT INTO research_retskew(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert retskew: {e}"))?;
    Ok(())
}

pub fn get_retskew(conn: &Connection, symbol: &str) -> Result<Option<ReturnSkewnessSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_retskew WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_retskew: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_retskew: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_retskew: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_retkurt(conn: &Connection, symbol: &str, snap: &ReturnKurtosisSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("retkurt json: {e}"))?;
    conn.execute(
        "INSERT INTO research_retkurt(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert retkurt: {e}"))?;
    Ok(())
}

pub fn get_retkurt(conn: &Connection, symbol: &str) -> Result<Option<ReturnKurtosisSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_retkurt WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_retkurt: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_retkurt: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_retkurt: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_tailr(conn: &Connection, symbol: &str, snap: &TailRatioSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("tailr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_tailr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert tailr: {e}"))?;
    Ok(())
}

pub fn get_tailr(conn: &Connection, symbol: &str) -> Result<Option<TailRatioSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_tailr WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_tailr: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_tailr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_tailr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_runlen(conn: &Connection, symbol: &str, snap: &RunLengthSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("runlen json: {e}"))?;
    conn.execute(
        "INSERT INTO research_runlen(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert runlen: {e}"))?;
    Ok(())
}

pub fn get_runlen(conn: &Connection, symbol: &str) -> Result<Option<RunLengthSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_runlen WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_runlen: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_runlen: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_runlen: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_dayrange(conn: &Connection, symbol: &str, snap: &DailyRangeSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("dayrange json: {e}"))?;
    conn.execute(
        "INSERT INTO research_dayrange(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert dayrange: {e}"))?;
    Ok(())
}

pub fn get_dayrange(conn: &Connection, symbol: &str) -> Result<Option<DailyRangeSnapshot>, String> {
    let _ = create_research_tables_v22(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_dayrange WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_dayrange: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_dayrange: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_dayrange: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-130 Web article ingestion (JSON-blob-per-symbol, schema v23) ──
//
// Agent-supplied web research articles. When the research packet's
// "Return Path" footer asks an AI agent to emit a fenced
// `===TYPHOON_INGEST===` block of article objects, the INGEST_RESEARCH
// command parses that block and merges the articles into the
// `research_web_articles` cache. LAN sync then distributes the
// ingested corpus to peer terminals.

/// One web research article captured from an AI agent's reply.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebArticle {
    pub title: String,
    pub url: String,
    pub source: String,        // publication / domain
    pub published_at: String,  // ISO-8601 preferred, any string tolerated
    pub summary: String,
    pub agent_used: String,    // "claude" | "gemini" | "chatgpt" | free-form
    pub ingested_at: i64,      // unix seconds
}

/// Per-symbol bag of ingested web articles. JSON-blob-per-symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestedArticlesSnapshot {
    pub symbol: String,
    pub articles: Vec<WebArticle>,
}

/// Max articles retained per symbol (FIFO drop by ingested_at).
pub const INGESTED_ARTICLES_MAX: usize = 50;

pub fn create_research_tables_v23(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_web_articles (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_web_articles_updated ON research_web_articles(updated_at);",
    ).map_err(|e| format!("create v23 tables: {e}"))?;
    Ok(())
}

pub fn upsert_ingested_articles(
    conn: &Connection,
    symbol: &str,
    snap: &IngestedArticlesSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v23(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ingested articles json: {e}"))?;
    conn.execute(
        "INSERT INTO research_web_articles(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ingested articles: {e}"))?;
    Ok(())
}

pub fn get_ingested_articles(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<IngestedArticlesSnapshot>, String> {
    let _ = create_research_tables_v23(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_web_articles WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ingested_articles: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_ingested_articles: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_ingested_articles: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

/// Merge new articles into the symbol's existing bag.
///
/// Dedupe by URL (case-insensitive). On conflict the newer entry wins
/// (articles with a larger `ingested_at` replace older ones). After
/// merging, the bag is trimmed to the latest `INGESTED_ARTICLES_MAX`
/// articles by `ingested_at` (most-recent first, FIFO drop of oldest).
/// Returns `(added_count, total_count)`.
pub fn append_ingested_articles(
    conn: &Connection,
    symbol: &str,
    incoming: Vec<WebArticle>,
) -> Result<(usize, usize), String> {
    let _ = create_research_tables_v23(conn);
    let mut existing = get_ingested_articles(conn, symbol)?
        .unwrap_or_else(|| IngestedArticlesSnapshot { symbol: symbol.to_uppercase(), articles: Vec::new() });

    let before = existing.articles.len();

    for mut art in incoming {
        if art.url.trim().is_empty() { continue; }
        if art.ingested_at == 0 { art.ingested_at = now_ts(); }
        let key = art.url.trim().to_lowercase();
        if let Some(pos) = existing.articles.iter().position(|a| a.url.trim().to_lowercase() == key) {
            if art.ingested_at >= existing.articles[pos].ingested_at {
                existing.articles[pos] = art;
            }
        } else {
            existing.articles.push(art);
        }
    }

    existing.articles.sort_by(|a, b| b.ingested_at.cmp(&a.ingested_at));
    if existing.articles.len() > INGESTED_ARTICLES_MAX {
        existing.articles.truncate(INGESTED_ARTICLES_MAX);
    }
    let after = existing.articles.len();
    let added = after.saturating_sub(before);

    upsert_ingested_articles(conn, symbol, &existing)?;
    Ok((added, after))
}

/// Parse one or more fenced `===TYPHOON_INGEST===` blocks out of an
/// AI agent reply and return them grouped by uppercase symbol.
///
/// Block format (the footer appended to research packets asks agents
/// to emit exactly this):
///
/// ```text
/// ===TYPHOON_INGEST===
/// [
///   {"symbol": "AAPL", "title": "...", "url": "...", "source": "...",
///    "published_at": "2026-04-15", "summary": "...", "agent": "claude"},
///   ...
/// ]
/// ===END_INGEST===
/// ```
///
/// The parser is lenient: it accepts `published` / `date` as aliases
/// for `published_at`, `agent` for `agent_used`, and silently skips
/// entries with no `url` or no `symbol`. It also tolerates surrounding
/// ```json fences and surrounding whitespace. The `ingested_at` field
/// is always set to the current timestamp at parse time.
pub fn parse_ingest_block(text: &str) -> Vec<(String, Vec<WebArticle>)> {
    let mut out: std::collections::BTreeMap<String, Vec<WebArticle>> = std::collections::BTreeMap::new();
    let now = now_ts();

    let mut rest = text;
    loop {
        let start = match rest.find("===TYPHOON_INGEST===") { Some(i) => i, None => break };
        let after_start = &rest[start + "===TYPHOON_INGEST===".len()..];
        let end_idx = match after_start.find("===END_INGEST===") { Some(i) => i, None => after_start.len() };
        let mut block = after_start[..end_idx].trim().to_string();

        // Strip ```json / ``` fences if present.
        if block.starts_with("```") {
            if let Some(nl) = block.find('\n') {
                block = block[nl + 1..].to_string();
            }
        }
        if block.ends_with("```") {
            let cut = block.len() - 3;
            block = block[..cut].trim_end().to_string();
        }

        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&block) {
            if let Some(arr) = v.as_array() {
                for item in arr {
                    let obj = match item.as_object() { Some(o) => o, None => continue };
                    let symbol = obj.get("symbol").and_then(|s| s.as_str()).unwrap_or("").trim().to_uppercase();
                    if symbol.is_empty() { continue; }
                    let url = obj.get("url").and_then(|s| s.as_str()).unwrap_or("").trim().to_string();
                    if url.is_empty() { continue; }
                    let title = obj.get("title").and_then(|s| s.as_str()).unwrap_or("").to_string();
                    let source = obj.get("source").and_then(|s| s.as_str()).unwrap_or("").to_string();
                    let published_at = obj.get("published_at").and_then(|s| s.as_str())
                        .or_else(|| obj.get("published").and_then(|s| s.as_str()))
                        .or_else(|| obj.get("date").and_then(|s| s.as_str()))
                        .unwrap_or("").to_string();
                    let summary = obj.get("summary").and_then(|s| s.as_str()).unwrap_or("").to_string();
                    let agent_used = obj.get("agent_used").and_then(|s| s.as_str())
                        .or_else(|| obj.get("agent").and_then(|s| s.as_str()))
                        .unwrap_or("").to_string();
                    out.entry(symbol).or_default().push(WebArticle {
                        title, url, source, published_at, summary, agent_used, ingested_at: now,
                    });
                }
            }
        }

        rest = &after_start[end_idx..];
        if rest.is_empty() { break; }
        if let Some(skip) = rest.find("===END_INGEST===") {
            rest = &rest[skip + "===END_INGEST===".len()..];
        } else {
            break;
        }
    }

    out.into_iter().collect()
}

// ── ADR-131 Godel Parity Round 23 schema + helpers ────────────────────────

pub fn create_research_tables_v24(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v23(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_autocor (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_autocor_updated ON research_autocor(updated_at);

        CREATE TABLE IF NOT EXISTS research_hurst (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hurst_updated ON research_hurst(updated_at);

        CREATE TABLE IF NOT EXISTS research_hitrate (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_hitrate_updated ON research_hitrate(updated_at);

        CREATE TABLE IF NOT EXISTS research_glasym (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_glasym_updated ON research_glasym(updated_at);

        CREATE TABLE IF NOT EXISTS research_volratio (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_volratio_updated ON research_volratio(updated_at);",
    ).map_err(|e| format!("create v24 tables: {e}"))?;
    Ok(())
}

pub fn upsert_autocor(conn: &Connection, symbol: &str, snap: &AutocorrelationSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("autocor json: {e}"))?;
    conn.execute(
        "INSERT INTO research_autocor(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert autocor: {e}"))?;
    Ok(())
}

pub fn get_autocor(conn: &Connection, symbol: &str) -> Result<Option<AutocorrelationSnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_autocor WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_autocor: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_autocor: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_autocor: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_hurst(conn: &Connection, symbol: &str, snap: &HurstSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hurst json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hurst(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hurst: {e}"))?;
    Ok(())
}

pub fn get_hurst(conn: &Connection, symbol: &str) -> Result<Option<HurstSnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_hurst WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hurst: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_hurst: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hurst: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_hitrate(conn: &Connection, symbol: &str, snap: &HitRateSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("hitrate json: {e}"))?;
    conn.execute(
        "INSERT INTO research_hitrate(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert hitrate: {e}"))?;
    Ok(())
}

pub fn get_hitrate(conn: &Connection, symbol: &str) -> Result<Option<HitRateSnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_hitrate WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_hitrate: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_hitrate: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_hitrate: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_glasym(conn: &Connection, symbol: &str, snap: &GainLossAsymmetrySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("glasym json: {e}"))?;
    conn.execute(
        "INSERT INTO research_glasym(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert glasym: {e}"))?;
    Ok(())
}

pub fn get_glasym(conn: &Connection, symbol: &str) -> Result<Option<GainLossAsymmetrySnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_glasym WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_glasym: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_glasym: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_glasym: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_volratio(conn: &Connection, symbol: &str, snap: &VolumeRatioSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("volratio json: {e}"))?;
    conn.execute(
        "INSERT INTO research_volratio(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert volratio: {e}"))?;
    Ok(())
}

pub fn get_volratio(conn: &Connection, symbol: &str) -> Result<Option<VolumeRatioSnapshot>, String> {
    let _ = create_research_tables_v24(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_volratio WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_volratio: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_volratio: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_volratio: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-132 Round 24 schema + upsert/get ──

pub fn create_research_tables_v25(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v24(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_drawup (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_drawup_updated ON research_drawup(updated_at);

        CREATE TABLE IF NOT EXISTS research_gapstats (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_gapstats_updated ON research_gapstats(updated_at);

        CREATE TABLE IF NOT EXISTS research_volcluster (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_volcluster_updated ON research_volcluster(updated_at);

        CREATE TABLE IF NOT EXISTS research_closeplc (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_closeplc_updated ON research_closeplc(updated_at);

        CREATE TABLE IF NOT EXISTS research_mrhl (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_mrhl_updated ON research_mrhl(updated_at);",
    ).map_err(|e| format!("create v25 tables: {e}"))?;
    Ok(())
}

pub fn upsert_drawup(conn: &Connection, symbol: &str, snap: &DrawupHistorySnapshot) -> Result<(), String> {
    let _ = create_research_tables_v25(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("drawup json: {e}"))?;
    conn.execute(
        "INSERT INTO research_drawup(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert drawup: {e}"))?;
    Ok(())
}

pub fn get_drawup(conn: &Connection, symbol: &str) -> Result<Option<DrawupHistorySnapshot>, String> {
    let _ = create_research_tables_v25(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_drawup WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_drawup: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_drawup: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_drawup: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_gapstats(conn: &Connection, symbol: &str, snap: &GapStatsSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v25(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("gapstats json: {e}"))?;
    conn.execute(
        "INSERT INTO research_gapstats(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert gapstats: {e}"))?;
    Ok(())
}

pub fn get_gapstats(conn: &Connection, symbol: &str) -> Result<Option<GapStatsSnapshot>, String> {
    let _ = create_research_tables_v25(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_gapstats WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_gapstats: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_gapstats: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_gapstats: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_volcluster(conn: &Connection, symbol: &str, snap: &VolClusterSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v25(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("volcluster json: {e}"))?;
    conn.execute(
        "INSERT INTO research_volcluster(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert volcluster: {e}"))?;
    Ok(())
}

pub fn get_volcluster(conn: &Connection, symbol: &str) -> Result<Option<VolClusterSnapshot>, String> {
    let _ = create_research_tables_v25(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_volcluster WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_volcluster: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_volcluster: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_volcluster: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_closeplc(conn: &Connection, symbol: &str, snap: &ClosePlacementSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v25(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("closeplc json: {e}"))?;
    conn.execute(
        "INSERT INTO research_closeplc(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert closeplc: {e}"))?;
    Ok(())
}

pub fn get_closeplc(conn: &Connection, symbol: &str) -> Result<Option<ClosePlacementSnapshot>, String> {
    let _ = create_research_tables_v25(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_closeplc WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_closeplc: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_closeplc: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_closeplc: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_mrhl(conn: &Connection, symbol: &str, snap: &MeanReversionHalfLifeSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v25(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("mrhl json: {e}"))?;
    conn.execute(
        "INSERT INTO research_mrhl(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert mrhl: {e}"))?;
    Ok(())
}

pub fn get_mrhl(conn: &Connection, symbol: &str) -> Result<Option<MeanReversionHalfLifeSnapshot>, String> {
    let _ = create_research_tables_v25(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_mrhl WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_mrhl: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_mrhl: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_mrhl: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

// ── ADR-133 Round 25 schema + upsert/get ──

pub fn create_research_tables_v26(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v25(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_downvol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_downvol_updated ON research_downvol(updated_at);

        CREATE TABLE IF NOT EXISTS research_sharpr (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_sharpr_updated ON research_sharpr(updated_at);

        CREATE TABLE IF NOT EXISTS research_effratio (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_effratio_updated ON research_effratio(updated_at);

        CREATE TABLE IF NOT EXISTS research_wickbias (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_wickbias_updated ON research_wickbias(updated_at);

        CREATE TABLE IF NOT EXISTS research_volofvol (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_volofvol_updated ON research_volofvol(updated_at);",
    ).map_err(|e| format!("create v26 tables: {e}"))?;
    Ok(())
}

pub fn upsert_downvol(conn: &Connection, symbol: &str, snap: &DownsideVolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v26(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("downvol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_downvol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert downvol: {e}"))?;
    Ok(())
}

pub fn get_downvol(conn: &Connection, symbol: &str) -> Result<Option<DownsideVolSnapshot>, String> {
    let _ = create_research_tables_v26(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_downvol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_downvol: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_downvol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_downvol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_sharpr(conn: &Connection, symbol: &str, snap: &SharpeRatioSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v26(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("sharpr json: {e}"))?;
    conn.execute(
        "INSERT INTO research_sharpr(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert sharpr: {e}"))?;
    Ok(())
}

pub fn get_sharpr(conn: &Connection, symbol: &str) -> Result<Option<SharpeRatioSnapshot>, String> {
    let _ = create_research_tables_v26(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_sharpr WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_sharpr: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_sharpr: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_sharpr: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_effratio(conn: &Connection, symbol: &str, snap: &EfficiencyRatioSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v26(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("effratio json: {e}"))?;
    conn.execute(
        "INSERT INTO research_effratio(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert effratio: {e}"))?;
    Ok(())
}

pub fn get_effratio(conn: &Connection, symbol: &str) -> Result<Option<EfficiencyRatioSnapshot>, String> {
    let _ = create_research_tables_v26(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_effratio WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_effratio: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_effratio: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_effratio: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_wickbias(conn: &Connection, symbol: &str, snap: &WickBiasSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v26(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("wickbias json: {e}"))?;
    conn.execute(
        "INSERT INTO research_wickbias(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert wickbias: {e}"))?;
    Ok(())
}

pub fn get_wickbias(conn: &Connection, symbol: &str) -> Result<Option<WickBiasSnapshot>, String> {
    let _ = create_research_tables_v26(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_wickbias WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_wickbias: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_wickbias: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_wickbias: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

pub fn upsert_volofvol(conn: &Connection, symbol: &str, snap: &VolOfVolSnapshot) -> Result<(), String> {
    let _ = create_research_tables_v26(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("volofvol json: {e}"))?;
    conn.execute(
        "INSERT INTO research_volofvol(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert volofvol: {e}"))?;
    Ok(())
}

pub fn get_volofvol(conn: &Connection, symbol: &str) -> Result<Option<VolOfVolSnapshot>, String> {
    let _ = create_research_tables_v26(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_volofvol WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_volofvol: {e}"))?;
    let mut r = stmt.query(params![symbol.to_uppercase()]).map_err(|e| format!("query get_volofvol: {e}"))?;
    if let Some(row) = r.next().map_err(|e| format!("row get_volofvol: {e}"))? {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else { Ok(None) }
}

/// Whole-table scan of `research_divg`. Used by DVDRANK.
pub fn get_all_divg(conn: &Connection) -> Result<Vec<DivgSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_divg")
        .map_err(|e| format!("prepare get_all_divg: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_divg: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<DivgSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_earm`. Used by EARMRANK.
pub fn get_all_earm(conn: &Connection) -> Result<Vec<EarmSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_earm")
        .map_err(|e| format!("prepare get_all_earm: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_earm: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<EarmSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

/// Whole-table scan of `research_updm`. Used by UPDGRANK.
pub fn get_all_updm(conn: &Connection) -> Result<Vec<UpdmSnapshot>, String> {
    let _ = create_research_tables_v12(conn);
    let mut stmt = conn.prepare("SELECT snapshot_json FROM research_updm")
        .map_err(|e| format!("prepare get_all_updm: {e}"))?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("query_map get_all_updm: {e}"))?
        .filter_map(|r| r.ok())
        .filter_map(|j| serde_json::from_str::<UpdmSnapshot>(&j).ok())
        .collect();
    Ok(rows)
}

// ── Tests ──────────────────────────────────────────────────────────────────


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commodities_universe_has_expected_sectors() {
        let sectors: std::collections::HashSet<&str> = COMMODITIES_UNIVERSE.iter().map(|(_, _, s)| *s).collect();
        assert!(sectors.contains("Metals"));
        assert!(sectors.contains("Energy"));
        assert!(sectors.contains("Grains"));
        assert!(sectors.contains("Softs"));
        assert!(sectors.contains("Livestock"));
    }

    #[test]
    fn commodities_universe_all_yahoo_futures_format() {
        for (sym, _, _) in COMMODITIES_UNIVERSE {
            assert!(sym.ends_with("=F"), "{} should end with =F", sym);
        }
    }

    #[test]
    fn company_profile_default_is_empty() {
        let p = CompanyProfile::default();
        assert!(p.symbol.is_empty());
        assert_eq!(p.market_cap, 0.0);
    }

    #[test]
    fn earning_row_all_optional() {
        let r = EarningRow::default();
        assert!(r.actual.is_none());
        assert!(r.estimate.is_none());
        assert!(r.surprise.is_none());
    }

    #[test]
    fn transcript_meta_roundtrip_json() {
        let m = TranscriptMeta { symbol: "AAPL".into(), quarter: 4, year: 2023, date: "2024-02-01".into() };
        let j = serde_json::to_string(&m).unwrap();
        let b: TranscriptMeta = serde_json::from_str(&j).unwrap();
        assert_eq!(b.symbol, "AAPL");
        assert_eq!(b.quarter, 4);
    }

    // ── ADR-109 ─────────────────────────────────────────────────────────

    fn open_mem_conn() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v2(&c).unwrap();
        c
    }

    #[test]
    fn dividend_record_roundtrip() {
        let c = open_mem_conn();
        let rows = vec![
            DividendRecord {
                ex_date: "2024-11-01".into(), pay_date: "2024-11-14".into(),
                record_date: "2024-11-04".into(), declaration_date: "2024-10-15".into(),
                amount: 0.24, adjusted_amount: 0.24, label: "Regular Cash".into(),
            },
        ];
        upsert_dividends(&c, "AAPL", &rows).unwrap();
        let got = get_dividends(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].amount, 0.24);
        assert_eq!(got[0].label, "Regular Cash");
    }

    #[test]
    fn earnings_estimate_roundtrip() {
        let c = open_mem_conn();
        let rows = vec![
            EarningsEstimate {
                date: "2025-12-31".into(),
                eps_avg: 2.45, eps_high: 2.60, eps_low: 2.30,
                revenue_avg: 123_000_000.0, revenue_high: 128_000_000.0, revenue_low: 118_000_000.0,
                num_analysts_eps: 12, num_analysts_rev: 12,
            },
        ];
        upsert_earnings_estimates(&c, "MSFT", &rows).unwrap();
        let got = get_earnings_estimates(&c, "MSFT").unwrap().unwrap();
        assert_eq!(got.len(), 1);
        assert!((got[0].eps_avg - 2.45).abs() < 1e-9);
        assert_eq!(got[0].num_analysts_eps, 12);
    }

    #[test]
    fn rating_change_roundtrip() {
        let c = open_mem_conn();
        let rows = vec![
            RatingChange {
                date: "2024-03-01".into(), symbol: "AAPL".into(),
                company: "Apple Inc.".into(), firm: "Morgan Stanley".into(),
                action: "upgrade".into(),
                from_grade: "Hold".into(), to_grade: "Buy".into(),
                price_target: 220.0,
            },
        ];
        upsert_rating_changes(&c, "AAPL", &rows).unwrap();
        let got = get_rating_changes(&c, "AAPL").unwrap().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].action, "upgrade");
        assert!((got[0].price_target - 220.0).abs() < 1e-9);
    }

    #[test]
    fn treasury_tenor_ladder_has_four_rungs() {
        let tenors: std::collections::HashSet<&str> = TREASURY_TENORS.iter().map(|(_, t)| *t).collect();
        assert!(tenors.contains("13W"));
        assert!(tenors.contains("5Y"));
        assert!(tenors.contains("10Y"));
        assert!(tenors.contains("30Y"));
    }

    #[test]
    fn treasury_yield_default_is_empty() {
        let y = TreasuryYield::default();
        assert!(y.tenor.is_empty());
        assert_eq!(y.yield_pct, 0.0);
    }

    #[test]
    fn dividend_upsert_overwrites() {
        let c = open_mem_conn();
        upsert_dividends(&c, "IBM", &[
            DividendRecord { ex_date: "2024-05-01".into(), amount: 1.66, ..Default::default() }
        ]).unwrap();
        upsert_dividends(&c, "IBM", &[
            DividendRecord { ex_date: "2024-05-01".into(), amount: 1.67, ..Default::default() },
            DividendRecord { ex_date: "2024-08-01".into(), amount: 1.67, ..Default::default() },
        ]).unwrap();
        let rows = get_dividends(&c, "IBM").unwrap().unwrap();
        assert_eq!(rows.len(), 2);
    }

    // ── ADR-110 ─────────────────────────────────────────────────────────

    fn open_mem_conn_v3() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v3(&c).unwrap();
        c
    }

    #[test]
    fn financials_bundle_default_is_empty() {
        let b = FinancialStatements::default();
        assert!(b.income_annual.is_empty());
        assert!(b.income_quarterly.is_empty());
        assert!(b.balance_annual.is_empty());
        assert!(b.balance_quarterly.is_empty());
        assert!(b.cashflow_annual.is_empty());
        assert!(b.cashflow_quarterly.is_empty());
    }

    #[test]
    fn financials_bundle_roundtrip() {
        let c = open_mem_conn_v3();
        let mut b = FinancialStatements::default();
        b.income_annual.push(IncomeStatement {
            date: "2024-09-30".into(), period: "FY".into(),
            revenue: 400_000_000_000.0, net_income: 97_000_000_000.0,
            ebitda: 135_000_000_000.0, eps: 6.12, eps_diluted: 6.08,
            ..Default::default()
        });
        b.balance_quarterly.push(BalanceSheet {
            date: "2024-06-30".into(), period: "Q3".into(),
            total_assets: 350_000_000_000.0, total_liabilities: 270_000_000_000.0,
            total_equity: 80_000_000_000.0, total_debt: 110_000_000_000.0,
            ..Default::default()
        });
        b.cashflow_annual.push(CashFlowStatement {
            date: "2024-09-30".into(), period: "FY".into(),
            cash_from_operations: 118_000_000_000.0, capex: -11_000_000_000.0,
            free_cash_flow: 107_000_000_000.0,
            ..Default::default()
        });
        upsert_financials(&c, "AAPL", &b).unwrap();
        let got = get_financials(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.income_annual.len(), 1);
        assert_eq!(got.balance_quarterly.len(), 1);
        assert_eq!(got.cashflow_annual.len(), 1);
        assert!((got.income_annual[0].eps - 6.12).abs() < 1e-9);
        assert!((got.cashflow_annual[0].free_cash_flow - 107_000_000_000.0).abs() < 1.0);
    }

    #[test]
    fn financials_upsert_replaces() {
        let c = open_mem_conn_v3();
        let mut b1 = FinancialStatements::default();
        b1.income_annual.push(IncomeStatement { date: "2023-09-30".into(), revenue: 1.0, ..Default::default() });
        upsert_financials(&c, "T", &b1).unwrap();
        let mut b2 = FinancialStatements::default();
        b2.income_annual.push(IncomeStatement { date: "2024-09-30".into(), revenue: 2.0, ..Default::default() });
        b2.income_annual.push(IncomeStatement { date: "2023-09-30".into(), revenue: 1.0, ..Default::default() });
        upsert_financials(&c, "T", &b2).unwrap();
        let got = get_financials(&c, "T").unwrap().unwrap();
        assert_eq!(got.income_annual.len(), 2);
    }

    #[test]
    fn executive_roundtrip() {
        let c = open_mem_conn_v3();
        let rows = vec![
            Executive {
                name: "Tim Cook".into(), position: "CEO".into(),
                age: 64, sex: "M".into(), since: "2011".into(),
                compensation: 74_600_000.0, year: 2023,
            },
            Executive {
                name: "Luca Maestri".into(), position: "CFO".into(),
                age: 60, sex: "M".into(), since: "2014".into(),
                compensation: 27_100_000.0, year: 2023,
            },
        ];
        upsert_executives(&c, "AAPL", &rows).unwrap();
        let got = get_executives(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].name, "Tim Cook");
        assert!((got[1].compensation - 27_100_000.0).abs() < 1.0);
    }

    #[test]
    fn cot_report_default_is_empty() {
        let r = CotReport::default();
        assert!(r.market_name.is_empty());
        assert_eq!(r.open_interest, 0.0);
        assert_eq!(r.noncomm_net, 0.0);
        assert_eq!(r.noncomm_net_change, 0.0);
    }

    #[test]
    fn cot_report_net_math() {
        // Derived invariant used by the UI's coloring / direction signal.
        let r = CotReport {
            noncomm_long: 120_000.0, noncomm_short: 45_000.0,
            noncomm_net: 120_000.0 - 45_000.0,
            noncomm_net_change: 5_000.0,
            ..Default::default()
        };
        assert!((r.noncomm_net - 75_000.0).abs() < 1e-9);
        assert!(r.noncomm_net_change > 0.0);
    }

    // ── ADR-111 ─────────────────────────────────────────────────────────

    fn open_mem_conn_v4() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v4(&c).unwrap();
        c
    }

    #[test]
    fn stock_split_default_is_empty() {
        let s = StockSplit::default();
        assert!(s.date.is_empty());
        assert!(s.label.is_empty());
        assert_eq!(s.numerator, 0.0);
        assert_eq!(s.denominator, 0.0);
    }

    #[test]
    fn stock_split_roundtrip() {
        let c = open_mem_conn_v4();
        let rows = vec![
            StockSplit { date: "2020-08-31".into(), label: "4:1".into(), numerator: 4.0, denominator: 1.0 },
            StockSplit { date: "2014-06-09".into(), label: "7:1".into(), numerator: 7.0, denominator: 1.0 },
        ];
        upsert_stock_splits(&c, "AAPL", &rows).unwrap();
        let got = get_stock_splits(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].label, "4:1");
        assert!((got[1].numerator - 7.0).abs() < 1e-9);
    }

    #[test]
    fn etf_holding_roundtrip() {
        let c = open_mem_conn_v4();
        let rows = vec![
            EtfHolding {
                symbol: "AAPL".into(), name: "Apple Inc.".into(),
                weight_pct: 7.21, shares: 176_000_000.0, market_value: 34_500_000_000.0,
                updated: "2024-06-30".into(),
            },
            EtfHolding {
                symbol: "MSFT".into(), name: "Microsoft Corp.".into(),
                weight_pct: 6.87, shares: 83_000_000.0, market_value: 32_900_000_000.0,
                updated: "2024-06-30".into(),
            },
        ];
        upsert_etf_holdings(&c, "SPY", &rows).unwrap();
        let got = get_etf_holdings(&c, "spy").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].symbol, "AAPL");
        assert!((got[1].weight_pct - 6.87).abs() < 1e-9);
    }

    #[test]
    fn analyst_rec_roundtrip() {
        let c = open_mem_conn_v4();
        let rows = vec![
            AnalystRecommendation {
                period: "2026-04-01".into(),
                strong_buy: 15, buy: 12, hold: 8, sell: 1, strong_sell: 0,
            },
            AnalystRecommendation {
                period: "2026-03-01".into(),
                strong_buy: 14, buy: 13, hold: 9, sell: 1, strong_sell: 0,
            },
        ];
        upsert_analyst_recs(&c, "AAPL", &rows).unwrap();
        let got = get_analyst_recs(&c, "AAPL").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].strong_buy, 15);
        assert_eq!(got[1].hold, 9);
    }

    #[test]
    fn price_target_default_is_empty() {
        let p = PriceTarget::default();
        assert!(p.symbol.is_empty());
        assert_eq!(p.target_mean, 0.0);
        assert_eq!(p.num_analysts, 0);
    }

    #[test]
    fn price_target_roundtrip() {
        let c = open_mem_conn_v4();
        let pt = PriceTarget {
            symbol: "NVDA".into(),
            target_high: 220.0, target_low: 140.0,
            target_mean: 185.50, target_median: 190.0,
            last_updated: "2026-04-10".into(),
            num_analysts: 45,
        };
        upsert_price_target(&c, "NVDA", &pt).unwrap();
        let got = get_price_target(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.num_analysts, 45);
        assert!((got.target_mean - 185.50).abs() < 1e-9);
    }

    #[test]
    fn price_target_upsert_replaces() {
        let c = open_mem_conn_v4();
        upsert_price_target(&c, "T", &PriceTarget {
            symbol: "T".into(), target_mean: 20.0, num_analysts: 10, ..Default::default()
        }).unwrap();
        upsert_price_target(&c, "T", &PriceTarget {
            symbol: "T".into(), target_mean: 22.5, num_analysts: 12, ..Default::default()
        }).unwrap();
        let got = get_price_target(&c, "T").unwrap().unwrap();
        assert_eq!(got.num_analysts, 12);
        assert!((got.target_mean - 22.5).abs() < 1e-9);
    }

    #[test]
    fn esg_roundtrip() {
        let c = open_mem_conn_v4();
        let rows = vec![
            EsgScore {
                symbol: "AAPL".into(),
                environmental_score: 78.5, social_score: 71.2, governance_score: 82.3,
                esg_score: 77.3, year: 2024,
            },
            EsgScore {
                symbol: "AAPL".into(),
                environmental_score: 76.0, social_score: 70.0, governance_score: 80.5,
                esg_score: 75.5, year: 2023,
            },
        ];
        upsert_esg(&c, "AAPL", &rows).unwrap();
        let got = get_esg(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].year, 2024);
        assert!((got[0].esg_score - 77.3).abs() < 1e-9);
    }

    #[test]
    fn index_member_roundtrip() {
        let c = open_mem_conn_v4();
        let rows = vec![
            IndexMember {
                index: "SP500".into(), symbol: "AAPL".into(), name: "Apple Inc.".into(),
                sector: "Information Technology".into(), sub_sector: "Technology Hardware".into(),
                headquarters: "Cupertino, CA".into(), date_added: "1982-11-30".into(),
            },
            IndexMember {
                index: "SP500".into(), symbol: "MSFT".into(), name: "Microsoft Corp.".into(),
                sector: "Information Technology".into(), sub_sector: "Software".into(),
                headquarters: "Redmond, WA".into(), date_added: "1994-06-01".into(),
            },
        ];
        upsert_index_members(&c, "SP500", &rows).unwrap();
        let got = get_index_members(&c, "sp500").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].symbol, "AAPL");
        assert_eq!(got[1].sector, "Information Technology");
    }

    // ── ADR-112 Round 5 ─────────────────────────────────────────────────

    fn open_mem_conn_v5() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v5(&c).unwrap();
        c
    }

    #[test]
    fn insider_trade_default_is_empty() {
        let t = InsiderTrade::default();
        assert!(t.reporting_name.is_empty());
        assert_eq!(t.shares, 0.0);
        assert_eq!(t.value_usd, 0.0);
    }

    #[test]
    fn insider_trade_roundtrip() {
        let c = open_mem_conn_v5();
        let rows = vec![
            InsiderTrade {
                filing_date: "2026-03-10".into(),
                transaction_date: "2026-03-08".into(),
                reporting_name: "Musk, Elon".into(),
                transaction_type: "S-Sale".into(),
                acquisition_disposition: "D".into(),
                shares: 150_000.0,
                price: 245.60,
                value_usd: 150_000.0 * 245.60,
                shares_owned_after: 411_000_000.0,
                link: "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK=0001318605".into(),
            },
            InsiderTrade {
                filing_date: "2026-02-11".into(),
                transaction_date: "2026-02-10".into(),
                reporting_name: "Taneja, Vaibhav".into(),
                transaction_type: "P-Purchase".into(),
                acquisition_disposition: "A".into(),
                shares: 2_500.0,
                price: 180.00,
                value_usd: 2_500.0 * 180.0,
                shares_owned_after: 42_000.0,
                link: "".into(),
            },
        ];
        upsert_insider_trades(&c, "TSLA", &rows).unwrap();
        let got = get_insider_trades(&c, "tsla").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].transaction_type, "S-Sale");
        assert_eq!(got[1].acquisition_disposition, "A");
        assert!((got[0].value_usd - 150_000.0 * 245.60).abs() < 1e-6);
    }

    #[test]
    fn institutional_holder_roundtrip() {
        let c = open_mem_conn_v5();
        let rows = vec![
            InstitutionalHolder {
                holder: "Vanguard Group Inc.".into(),
                shares: 1_200_000_000.0,
                date_reported: "2025-12-31".into(),
                change: 12_000_000.0,
            },
            InstitutionalHolder {
                holder: "BlackRock Inc.".into(),
                shares: 1_050_000_000.0,
                date_reported: "2025-12-31".into(),
                change: -4_500_000.0,
            },
        ];
        upsert_institutional_holders(&c, "AAPL", &rows).unwrap();
        let got = get_institutional_holders(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].holder, "Vanguard Group Inc.");
        assert!(got[1].change < 0.0);
    }

    #[test]
    fn shares_float_default_is_empty() {
        let f = SharesFloat::default();
        assert!(f.symbol.is_empty());
        assert_eq!(f.free_float_pct, 0.0);
        assert_eq!(f.outstanding_shares, 0.0);
    }

    #[test]
    fn shares_float_roundtrip() {
        let c = open_mem_conn_v5();
        let snap = SharesFloat {
            symbol: "NVDA".into(),
            date: "2026-04-01".into(),
            free_float_pct: 95.8,
            float_shares: 23_500_000_000.0,
            outstanding_shares: 24_530_000_000.0,
            source: "FMP".into(),
        };
        upsert_shares_float(&c, "NVDA", &snap).unwrap();
        let got = get_shares_float(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.symbol, "NVDA");
        assert!((got.free_float_pct - 95.8).abs() < 1e-9);
        assert!((got.outstanding_shares - 24_530_000_000.0).abs() < 1.0);
    }

    #[test]
    fn historical_price_roundtrip() {
        let c = open_mem_conn_v5();
        let rows = vec![
            HistoricalPriceRow {
                date: "2026-04-13".into(),
                open: 180.0, high: 183.5, low: 179.2, close: 182.9,
                adj_close: 182.9, volume: 48_500_000.0,
                change: 2.9, change_pct: 1.61,
            },
            HistoricalPriceRow {
                date: "2026-04-12".into(),
                open: 178.1, high: 180.4, low: 177.8, close: 180.0,
                adj_close: 180.0, volume: 42_100_000.0,
                change: 1.9, change_pct: 1.07,
            },
        ];
        upsert_historical_price(&c, "AAPL", &rows).unwrap();
        let got = get_historical_price(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].date, "2026-04-13");
        assert!((got[0].change_pct - 1.61).abs() < 1e-9);
    }

    #[test]
    fn earnings_surprise_roundtrip() {
        let c = open_mem_conn_v5();
        let rows = vec![
            EarningsSurprise {
                date: "2026-02-01".into(),
                symbol: "AAPL".into(),
                eps_actual: 2.18,
                eps_estimate: 2.11,
                surprise: 0.07,
                surprise_pct: (0.07 / 2.11) * 100.0,
            },
            EarningsSurprise {
                date: "2025-11-01".into(),
                symbol: "AAPL".into(),
                eps_actual: 1.64,
                eps_estimate: 1.60,
                surprise: 0.04,
                surprise_pct: (0.04 / 1.60) * 100.0,
            },
        ];
        upsert_earnings_surprises(&c, "AAPL", &rows).unwrap();
        let got = get_earnings_surprises(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert!(got[0].surprise > 0.0);
        assert!((got[0].surprise_pct - (0.07 / 2.11) * 100.0).abs() < 1e-9);
    }

    #[test]
    fn earnings_surprise_upsert_replaces() {
        let c = open_mem_conn_v5();
        upsert_earnings_surprises(&c, "T", &[
            EarningsSurprise { date: "2025-10-01".into(), symbol: "T".into(),
                eps_actual: 0.55, eps_estimate: 0.58, surprise: -0.03, surprise_pct: -5.17 }
        ]).unwrap();
        upsert_earnings_surprises(&c, "T", &[
            EarningsSurprise { date: "2026-01-01".into(), symbol: "T".into(),
                eps_actual: 0.60, eps_estimate: 0.57, surprise: 0.03, surprise_pct: 5.26 }
        ]).unwrap();
        let got = get_earnings_surprises(&c, "T").unwrap().unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].date, "2026-01-01");
        assert!(got[0].surprise > 0.0);
    }

    // ── ADR-113 Round 6 ─────────────────────────────────────────────────

    fn open_mem_conn_v6() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v6(&c).unwrap();
        c
    }

    #[test]
    fn world_indices_universe_has_all_regions() {
        let regions: std::collections::HashSet<&str> =
            WORLD_INDICES_UNIVERSE.iter().map(|(_, _, r)| *r).collect();
        assert!(regions.contains("Americas"));
        assert!(regions.contains("EMEA"));
        assert!(regions.contains("Asia-Pacific"));
    }

    #[test]
    fn world_indices_universe_has_sp500_and_nikkei() {
        let tickers: std::collections::HashSet<&str> =
            WORLD_INDICES_UNIVERSE.iter().map(|(t, _, _)| *t).collect();
        assert!(tickers.contains("^GSPC"));
        assert!(tickers.contains("^N225"));
        assert!(tickers.contains("^FTSE"));
    }

    #[test]
    fn world_indices_roundtrip() {
        let c = open_mem_conn_v6();
        let rows = vec![
            WorldIndex { ticker: "^GSPC".into(), display: "S&P 500".into(), region: "Americas".into(),
                price: 5200.0, change: 12.5, change_pct: 0.24 },
            WorldIndex { ticker: "^N225".into(), display: "Nikkei 225".into(), region: "Asia-Pacific".into(),
                price: 39_800.0, change: -150.0, change_pct: -0.38 },
        ];
        upsert_world_indices(&c, &rows).unwrap();
        let got = get_world_indices(&c).unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].ticker, "^GSPC");
        assert!(got[1].change < 0.0);
    }

    #[test]
    fn world_indices_upsert_replaces() {
        let c = open_mem_conn_v6();
        upsert_world_indices(&c, &[
            WorldIndex { ticker: "^GSPC".into(), price: 5000.0, ..Default::default() },
        ]).unwrap();
        upsert_world_indices(&c, &[
            WorldIndex { ticker: "^GSPC".into(), price: 5300.0, ..Default::default() },
            WorldIndex { ticker: "^DJI".into(), price: 42_000.0, ..Default::default() },
        ]).unwrap();
        let got = get_world_indices(&c).unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert!((got[0].price - 5300.0).abs() < 1e-9);
    }

    #[test]
    fn market_movers_roundtrip() {
        let c = open_mem_conn_v6();
        let movers = MarketMovers {
            gainers: vec![
                MarketMover { symbol: "AAA".into(), name: "Alpha Inc.".into(),
                    price: 12.5, change: 2.1, change_pct: 20.19, volume: 1_200_000.0 },
            ],
            losers: vec![
                MarketMover { symbol: "ZZZ".into(), name: "Omega Corp.".into(),
                    price: 4.8, change: -1.1, change_pct: -18.64, volume: 900_000.0 },
            ],
            actives: vec![
                MarketMover { symbol: "TSLA".into(), name: "Tesla Inc.".into(),
                    price: 190.25, change: 1.15, change_pct: 0.61, volume: 120_000_000.0 },
            ],
        };
        upsert_market_movers(&c, &movers).unwrap();
        let got = get_market_movers(&c).unwrap().unwrap();
        assert_eq!(got.gainers.len(), 1);
        assert_eq!(got.losers.len(), 1);
        assert_eq!(got.actives.len(), 1);
        assert_eq!(got.gainers[0].symbol, "AAA");
        assert!(got.losers[0].change_pct < 0.0);
        assert_eq!(got.actives[0].symbol, "TSLA");
    }

    #[test]
    fn sector_performance_roundtrip() {
        let c = open_mem_conn_v6();
        let rows = vec![
            SectorPerformance { sector: "Technology".into(),      change_pct: 1.23 },
            SectorPerformance { sector: "Energy".into(),          change_pct: -0.45 },
            SectorPerformance { sector: "Financial Services".into(), change_pct: 0.78 },
        ];
        upsert_sector_performance(&c, &rows).unwrap();
        let got = get_sector_performance(&c).unwrap().unwrap();
        assert_eq!(got.len(), 3);
        assert_eq!(got[0].sector, "Technology");
        assert!(got[1].change_pct < 0.0);
    }

    #[test]
    fn wacc_compute_basic_calc() {
        let s = compute_wacc_snapshot(
            "AAPL", "2026-04-14",
            1.20,              // beta
            3_000_000_000_000.0, // market cap (3T)
            4.50,              // Rf %
            100_000_000_000.0, // total debt (100B)
            5_000_000_000.0,  // interest expense (5B)
            16.0,              // effective tax rate %
        );
        // Cost of equity = 4.5 + 1.20 * 5.0 = 10.5 %
        assert!((s.cost_of_equity_pct - 10.5).abs() < 1e-6);
        // Pre-tax cost of debt = (5B / 100B) * 100 = 5.0 %
        assert!((s.pre_tax_cost_of_debt_pct - 5.0).abs() < 1e-6);
        // After-tax = 5.0 * (1 - 0.16) = 4.2 %
        assert!((s.after_tax_cost_of_debt_pct - 4.2).abs() < 1e-6);
        // Weights: E=3T / (3T+100B) ≈ 0.9677, D ≈ 0.0323
        assert!((s.equity_weight - 3000.0/3100.0).abs() < 1e-6);
        // WACC ≈ 0.9677*10.5 + 0.0323*4.2 ≈ 10.296
        let expected = (3000.0/3100.0)*10.5 + (100.0/3100.0)*4.2;
        assert!((s.wacc_pct - expected).abs() < 1e-6);
    }

    #[test]
    fn wacc_handles_zero_debt() {
        let s = compute_wacc_snapshot(
            "NVDA", "2026-04-14",
            1.80,   // beta
            2_500_000_000_000.0, // market cap
            4.30,   // Rf
            0.0,    // no debt
            0.0,    // no interest expense
            12.0,   // tax
        );
        assert_eq!(s.pre_tax_cost_of_debt_pct, 0.0);
        assert_eq!(s.debt_weight, 0.0);
        assert!((s.equity_weight - 1.0).abs() < 1e-9);
        // WACC == Re when all equity
        assert!((s.wacc_pct - s.cost_of_equity_pct).abs() < 1e-9);
    }

    #[test]
    fn wacc_roundtrip() {
        let c = open_mem_conn_v6();
        let snap = compute_wacc_snapshot("AAPL", "2026-04-14",
            1.20, 3_000_000_000_000.0, 4.50,
            100_000_000_000.0, 5_000_000_000.0, 16.0);
        upsert_wacc(&c, "AAPL", &snap).unwrap();
        let got = get_wacc(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert!((got.wacc_pct - snap.wacc_pct).abs() < 1e-9);
        assert!((got.beta - 1.20).abs() < 1e-9);
    }

    #[test]
    fn fmp_mover_parses_string_percentage() {
        // FMP sometimes returns changesPercentage as "1.23%" (string), sometimes as f64.
        let v: serde_json::Value = serde_json::from_str(r#"{
            "symbol":"AAPL","name":"Apple","price":185.5,"change":2.1,
            "changesPercentage":"1.15%","volume":45000000
        }"#).unwrap();
        let m = parse_fmp_mover(&v);
        assert_eq!(m.symbol, "AAPL");
        assert!((m.change_pct - 1.15).abs() < 1e-9);
        assert!((m.volume - 45_000_000.0).abs() < 1.0);
    }

    // ── ADR-114 Round 7 ─────────────────────────────────────────────────

    fn open_mem_conn_v7() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v7(&c).unwrap();
        c
    }

    #[test]
    fn fx_majors_universe_has_regions() {
        let regions: std::collections::HashSet<&str> =
            FX_MAJORS_UNIVERSE.iter().map(|(_, _, _, _, r)| *r).collect();
        assert!(regions.contains("Majors"));
        assert!(regions.contains("Crosses"));
        assert!(regions.contains("EM"));
    }

    #[test]
    fn fx_majors_universe_has_eurusd_and_usdjpy() {
        let tickers: std::collections::HashSet<&str> =
            FX_MAJORS_UNIVERSE.iter().map(|(t, _, _, _, _)| *t).collect();
        assert!(tickers.contains("EURUSD=X"));
        assert!(tickers.contains("USDJPY=X"));
        assert!(tickers.contains("USDMXN=X"));
    }

    #[test]
    fn currency_rates_roundtrip() {
        let c = open_mem_conn_v7();
        let rows = vec![
            CurrencyRate {
                ticker: "EURUSD=X".into(), display: "EUR/USD".into(),
                base: "EUR".into(), quote: "USD".into(), region: "Majors".into(),
                price: 1.0850, change: 0.0020, change_pct: 0.18,
            },
            CurrencyRate {
                ticker: "USDJPY=X".into(), display: "USD/JPY".into(),
                base: "USD".into(), quote: "JPY".into(), region: "Majors".into(),
                price: 151.25, change: -0.35, change_pct: -0.23,
            },
        ];
        upsert_currency_rates(&c, &rows).unwrap();
        let got = get_currency_rates(&c).unwrap().unwrap();
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].display, "EUR/USD");
        assert!(got[1].change < 0.0);
    }

    #[test]
    fn ols_regression_perfect_correlation() {
        // If s_i == 2 * m_i exactly, beta should be exactly 2.0, R² = 1.
        let m: Vec<f64> = vec![0.01, -0.005, 0.02, -0.01, 0.015, 0.008, -0.003, 0.012, 0.005, -0.007, 0.018, -0.002];
        let s: Vec<f64> = m.iter().map(|x| 2.0 * x).collect();
        let (beta, _alpha, r2, corr, n) = ols_regression(&s, &m);
        assert!((beta - 2.0).abs() < 1e-9);
        assert!((r2 - 1.0).abs() < 1e-9);
        assert!((corr - 1.0).abs() < 1e-9);
        assert_eq!(n, 12);
    }

    #[test]
    fn compute_beta_snapshot_synthetic_2x_market() {
        // Build symbol bars that exactly track 2× market moves. Expected β ≈ 2.0.
        // Use 300 bars so the 1Y window (252) fits with headroom. FMP order is
        // newest-first, so we build newest → oldest. Dates must be unique —
        // we use a simple days-since-epoch counter so the join by date key
        // does not collide.
        let mut sym_bars: Vec<HistoricalPriceRow> = Vec::new();
        let mut mkt_bars: Vec<HistoricalPriceRow> = Vec::new();
        let mut sym_close = 100.0_f64;
        let mut mkt_close = 400.0_f64;
        for i in 0..300 {
            let daily = 0.01 * ((i as f64 * 0.37).sin());
            mkt_close *= 1.0 + daily;
            sym_close *= 1.0 + 2.0 * daily;
            // Fake-but-unique ISO date: walk calendar by 1-day increments from 2024-01-01.
            let base_day = 1 + (i % 28); // 1..=28
            let month = 1 + ((i / 28) % 12); // 1..=12
            let year = 2024 + ((i / (28 * 12)) as i32);
            let date = format!("{:04}-{:02}-{:02}", year, month, base_day);
            sym_bars.push(HistoricalPriceRow {
                date: date.clone(), open: sym_close, high: sym_close, low: sym_close,
                close: sym_close, adj_close: sym_close, volume: 0.0, change: 0.0, change_pct: 0.0,
            });
            mkt_bars.push(HistoricalPriceRow {
                date, open: mkt_close, high: mkt_close, low: mkt_close,
                close: mkt_close, adj_close: mkt_close, volume: 0.0, change: 0.0, change_pct: 0.0,
            });
        }
        // The loop already pushes in synthetic chronological order — we need
        // FMP's newest-first orientation, so reverse.
        sym_bars.reverse();
        mkt_bars.reverse();
        let snap = compute_beta_snapshot("TST", "SPY", "2026-04-14", &sym_bars, &mkt_bars);
        assert!(!snap.windows.is_empty());
        let w1y = snap.windows.iter().find(|w| w.window_label == "1Y").unwrap();
        assert!((w1y.beta - 2.0).abs() < 0.01, "beta was {}", w1y.beta);
        assert!(w1y.r_squared > 0.99);
    }

    #[test]
    fn beta_snapshot_roundtrip() {
        let c = open_mem_conn_v7();
        let snap = BetaSnapshot {
            symbol: "AAPL".into(),
            market_ticker: "SPY".into(),
            as_of: "2026-04-14".into(),
            windows: vec![
                BetaWindow { window_label: "1Y".into(), window_days: 252,
                    beta: 1.18, alpha_pct: 2.4, r_squared: 0.67,
                    n_observations: 252, correlation: 0.82 },
                BetaWindow { window_label: "5Y".into(), window_days: 1260,
                    beta: 1.23, alpha_pct: 4.1, r_squared: 0.71,
                    n_observations: 1260, correlation: 0.84 },
            ],
            note: String::new(),
        };
        upsert_beta(&c, "AAPL", &snap).unwrap();
        let got = get_beta(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.windows.len(), 2);
        assert!((got.windows[0].beta - 1.18).abs() < 1e-9);
    }

    #[test]
    fn compute_ddm_basic_growth() {
        // 10 years of dividends with 7% annual growth, required return 12% → finite price.
        let mut divs: Vec<DividendRecord> = Vec::new();
        let base = 1.00_f64;
        for y in 2016..=2025 {
            let growth = 1.07_f64.powi(y - 2016);
            for q in 1..=4 {
                divs.push(DividendRecord {
                    ex_date: format!("{}-{:02}-15", y, 1 + (q - 1) * 3),
                    pay_date: format!("{}-{:02}-28", y, 1 + (q - 1) * 3),
                    record_date: String::new(), declaration_date: String::new(),
                    amount: base * growth * 0.25,
                    adjusted_amount: base * growth * 0.25,
                    label: "Regular Cash".into(),
                });
            }
        }
        // Newest-first: sort descending by ex_date.
        divs.sort_by(|a, b| b.ex_date.cmp(&a.ex_date));
        let snap = compute_ddm_snapshot("AAA", "2026-04-14", &divs, 12.0, "WACC 12%");
        assert!(snap.annual_dividend > 0.0);
        assert!(snap.implied_growth_pct > 4.0 && snap.implied_growth_pct < 10.0,
            "growth was {}", snap.implied_growth_pct);
        assert!(snap.implied_price > 0.0);
        assert!(snap.note.is_empty());
    }

    #[test]
    fn compute_ddm_diverges_when_growth_exceeds_return() {
        let divs = vec![
            DividendRecord { ex_date: "2025-01-15".into(), amount: 1.0, adjusted_amount: 1.0, ..Default::default() },
            DividendRecord { ex_date: "2024-01-15".into(), amount: 0.80, adjusted_amount: 0.80, ..Default::default() },
            DividendRecord { ex_date: "2023-01-15".into(), amount: 0.60, adjusted_amount: 0.60, ..Default::default() },
            DividendRecord { ex_date: "2022-01-15".into(), amount: 0.45, adjusted_amount: 0.45, ..Default::default() },
        ];
        // Ask for very low required return — Gordon must diverge.
        let snap = compute_ddm_snapshot("BBB", "2026-04-14", &divs, 2.0, "manual");
        assert_eq!(snap.implied_price, 0.0);
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn ddm_roundtrip() {
        let c = open_mem_conn_v7();
        let snap = DdmSnapshot {
            symbol: "KO".into(),
            as_of: "2026-04-14".into(),
            annual_dividend: 1.92,
            implied_growth_pct: 4.5,
            required_return_pct: 8.0,
            growth_source: "5Y dividend CAGR".into(),
            return_source: "WACC 8.0%".into(),
            implied_price: 57.34,
            method: "Gordon Growth".into(),
            note: String::new(),
        };
        upsert_ddm(&c, "KO", &snap).unwrap();
        let got = get_ddm(&c, "ko").unwrap().unwrap();
        assert_eq!(got.symbol, "KO");
        assert!((got.implied_price - 57.34).abs() < 1e-9);
    }

    #[test]
    fn compute_relative_valuation_z_scores() {
        let inputs = vec![
            RvMetricInput {
                metric: "P/E",
                value: Some(30.0),
                peer_values: vec![10.0, 15.0, 20.0, 25.0, 28.0, 35.0, 40.0],
            },
            RvMetricInput {
                metric: "P/B",
                value: None, // should skip
                peer_values: vec![1.0, 2.0, 3.0, 4.0],
            },
            RvMetricInput {
                metric: "EV/EBITDA",
                value: Some(12.0),
                peer_values: vec![8.0, 10.0], // <3 peers — should skip
            },
        ];
        let rv = compute_relative_valuation("SUBJ", "Tech", "2026-04-14", &inputs);
        assert_eq!(rv.rows.len(), 1);
        let pe = &rv.rows[0];
        assert_eq!(pe.metric, "P/E");
        assert_eq!(pe.peer_low, 10.0);
        assert_eq!(pe.peer_high, 40.0);
        // 30 is higher than 5 of 7 peers → percentile ≈ 71.4
        assert!(pe.percentile > 60.0 && pe.percentile < 80.0);
        assert!(pe.z_score > 0.0); // above mean
    }

    #[test]
    fn relative_valuation_roundtrip() {
        let c = open_mem_conn_v7();
        let rv = RelativeValuation {
            symbol: "AAPL".into(),
            sector: "Technology".into(),
            as_of: "2026-04-14".into(),
            peer_count: 8,
            rows: vec![
                RvMetricRow { metric: "P/E".into(), value: 32.0, peer_median: 28.0,
                    peer_low: 12.0, peer_high: 60.0, z_score: 0.4, percentile: 62.5 },
            ],
        };
        upsert_relative_valuation(&c, "AAPL", &rv).unwrap();
        let got = get_relative_valuation(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.rows.len(), 1);
        assert!((got.rows[0].value - 32.0).abs() < 1e-9);
    }

    #[test]
    fn figi_roundtrip() {
        let c = open_mem_conn_v7();
        let snap = FigiSnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            identifiers: vec![
                FigiIdentifier {
                    figi: "BBG000B9XRY4".into(),
                    name: "APPLE INC".into(),
                    ticker: "AAPL".into(),
                    exch_code: "US".into(),
                    composite_figi: "BBG000B9Y5X2".into(),
                    share_class_figi: "BBG001S5N8V8".into(),
                    security_type: "Common Stock".into(),
                    security_type_2: "Common Stock".into(),
                    market_sector: "Equity".into(),
                    security_description: "AAPL".into(),
                },
            ],
        };
        upsert_figi(&c, "AAPL", &snap).unwrap();
        let got = get_figi(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.identifiers.len(), 1);
        assert_eq!(got.identifiers[0].figi, "BBG000B9XRY4");
    }

    // ── ADR-115 Round 8 tests ──────────────────────────────────────────

    fn open_mem_conn_v8() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v8(&c).unwrap();
        c
    }

    #[test]
    fn hra_roundtrip() {
        let c = open_mem_conn_v8();
        let snap = HraSnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            last_close: 190.0,
            windows: vec![HraWindow { label: "1Y".into(), trading_days: 252, return_pct: 22.0, cagr_pct: 22.0, n_observations: 252 }],
            max_drawdown_pct: -15.5,
            drawdown_peak_date: "2025-10-01".into(),
            drawdown_trough_date: "2025-12-15".into(),
            volatility_annual_pct: 24.0,
            sharpe_ratio: 1.1,
            sortino_ratio: 1.4,
            calmar_ratio: 1.4,
            risk_free_pct: 4.5,
            note: String::new(),
        };
        upsert_hra(&c, "AAPL", &snap).unwrap();
        let got = get_hra(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.windows.len(), 1);
        assert!((got.max_drawdown_pct - (-15.5)).abs() < 1e-6);
    }

    #[test]
    fn dcf_roundtrip() {
        let c = open_mem_conn_v8();
        let snap = DcfSnapshot {
            symbol: "NVDA".into(),
            as_of: "2026-04-14".into(),
            method: "DCF on FCFF".into(),
            base_revenue: 60_000.0,
            base_fcff: 24_000.0,
            growth_pct: 20.0,
            terminal_growth_pct: 3.0,
            wacc_pct: 9.0,
            tax_rate_pct: 15.0,
            fcff_margin_pct: 40.0,
            projection_years: 5,
            years: Vec::new(),
            pv_sum: 100_000.0,
            terminal_value: 500_000.0,
            pv_terminal: 350_000.0,
            enterprise_value: 450_000.0,
            total_debt: 10_000.0,
            cash_and_equivalents: 30_000.0,
            equity_value: 470_000.0,
            shares_outstanding: 2_500.0,
            implied_price: 188.0,
            note: String::new(),
        };
        upsert_dcf(&c, "NVDA", &snap).unwrap();
        let got = get_dcf(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.symbol, "NVDA");
        assert!((got.implied_price - 188.0).abs() < 1e-6);
    }

    #[test]
    fn svm_roundtrip() {
        let c = open_mem_conn_v8();
        let snap = SvmSnapshot {
            symbol: "MSFT".into(),
            as_of: "2026-04-14".into(),
            current_price: 420.0,
            rows: vec![SvmModelRow {
                model: "DCF on FCFF".into(),
                implied_price: 450.0,
                current_price: 420.0,
                upside_pct: 7.14,
                confidence: "medium".into(),
                source: "test".into(),
            }],
            fair_low: 450.0, fair_high: 450.0, fair_mid: 450.0,
            upside_mid_pct: 7.14,
            note: String::new(),
        };
        upsert_svm(&c, "MSFT", &snap).unwrap();
        let got = get_svm(&c, "msft").unwrap().unwrap();
        assert_eq!(got.rows.len(), 1);
        assert!((got.fair_mid - 450.0).abs() < 1e-6);
    }

    #[test]
    fn options_chain_roundtrip() {
        let c = open_mem_conn_v8();
        let snap = OptionsChainSnapshot {
            symbol: "SPY".into(),
            as_of: "2026-04-14".into(),
            underlying_price: 520.0,
            expirations: vec![OptionExpiry {
                expiration: "2026-05-16".into(),
                days_to_expiry: 32,
                calls: vec![OptionContract {
                    contract_symbol: "SPY260516C00520000".into(),
                    option_type: "CALL".into(),
                    strike: 520.0,
                    last_price: 8.5, bid: 8.4, ask: 8.6,
                    volume: 1200.0, open_interest: 5000.0,
                    implied_volatility: 0.18, in_the_money: false,
                }],
                puts: vec![],
            }],
            note: String::new(),
        };
        upsert_options_chain(&c, "SPY", &snap).unwrap();
        let got = get_options_chain(&c, "spy").unwrap().unwrap();
        assert_eq!(got.expirations.len(), 1);
        assert_eq!(got.expirations[0].calls.len(), 1);
        assert!((got.expirations[0].calls[0].strike - 520.0).abs() < 1e-6);
    }

    #[test]
    fn ivol_roundtrip() {
        let c = open_mem_conn_v8();
        let snap = IvolSnapshot {
            symbol: "TSLA".into(),
            as_of: "2026-04-14".into(),
            current_atm_iv_pct: 55.0,
            iv_52w_low_pct: 30.0,
            iv_52w_high_pct: 80.0,
            iv_rank: 50.0,
            iv_percentile: 60.0,
            observation_count: 100,
            history: vec![IvolObservation { date: "2026-01-01".into(), atm_iv_pct: 40.0 }],
            note: String::new(),
        };
        upsert_ivol(&c, "TSLA", &snap).unwrap();
        let got = get_ivol(&c, "tsla").unwrap().unwrap();
        assert!((got.iv_rank - 50.0).abs() < 1e-6);
        assert_eq!(got.history.len(), 1);
    }

    #[test]
    fn compute_hra_on_synthetic_uptrend() {
        // 300 daily bars, +0.1% per day → terminal ~ 1.001^299.
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        let mut px = 100.0;
        for i in 0..300 {
            let base_day = 1 + (i % 28);
            let month    = 1 + ((i / 28) % 12);
            let year     = 2024 + (i / (28 * 12));
            bars.push(HistoricalPriceRow {
                date: format!("{:04}-{:02}-{:02}", year, month, base_day),
                open: px, high: px, low: px, close: px, adj_close: px,
                volume: 1_000.0, change: 0.0, change_pct: 0.1,
            });
            px *= 1.001;
        }
        let snap = compute_hra_snapshot("TEST", "2026-04-14", &bars, 4.5);
        assert_eq!(snap.symbol, "TEST");
        // 1Y window present
        assert!(snap.windows.iter().any(|w| w.label == "1Y"));
        // ITD should be strongly positive
        let itd = snap.windows.iter().find(|w| w.label == "ITD").unwrap();
        assert!(itd.return_pct > 0.0);
        // Monotonic uptrend → drawdown effectively zero (we accept very small
        // rounding-scale negatives).
        assert!(snap.max_drawdown_pct > -0.1, "expected near-zero drawdown on monotonic uptrend, got {}", snap.max_drawdown_pct);
    }

    #[test]
    fn compute_hra_on_empty_bars_returns_note() {
        let snap = compute_hra_snapshot("EMPTY", "2026-04-14", &[], 4.5);
        assert!(!snap.note.is_empty());
        assert_eq!(snap.windows.len(), 0);
    }

    #[test]
    fn compute_hra_drawdown_detects_peak_and_trough() {
        // 50 bars that rise to 150 at day 20, then fall to 100 by day 40, then
        // recover to 130 at day 49. Max DD is from peak 150 to trough 100.
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        let mut push = |i: usize, close: f64| {
            let base_day = 1 + (i % 28);
            let month    = 1 + ((i / 28) % 12);
            let year     = 2024 + (i / (28 * 12));
            bars.push(HistoricalPriceRow {
                date: format!("{:04}-{:02}-{:02}", year, month, base_day),
                open: close, high: close, low: close, close, adj_close: close,
                volume: 1_000.0, change: 0.0, change_pct: 0.0,
            });
        };
        for i in 0..20 { push(i, 100.0 + (i as f64 * 2.5)); } // 100 → 147.5
        push(20, 150.0);                                      // peak
        for i in 21..=40 { push(i, 150.0 - ((i - 20) as f64 * 2.5)); } // 150 → 100
        for i in 41..50 { push(i, 100.0 + ((i - 40) as f64 * 3.333)); } // 100 → 130
        let snap = compute_hra_snapshot("X", "2026-04-14", &bars, 0.0);
        // Peak-to-trough 150→100 = -33.33%
        assert!(snap.max_drawdown_pct < -32.0 && snap.max_drawdown_pct > -34.0,
            "expected ~-33% drawdown, got {:.2}", snap.max_drawdown_pct);
    }

    #[test]
    fn compute_dcf_basic() {
        let snap = compute_dcf_snapshot("NVDA", "2026-04-14",
            /*revenue*/ 60_000.0, /*fcff*/ 24_000.0,
            /*g*/ 20.0, /*tg*/ 3.0, /*wacc*/ 9.0, /*tax*/ 15.0,
            /*years*/ 5, /*debt*/ 10_000.0, /*cash*/ 30_000.0,
            /*shares*/ 2_500.0);
        assert_eq!(snap.years.len(), 5);
        assert!(snap.enterprise_value > 0.0);
        assert!(snap.implied_price > 0.0);
        // Each projection year's fcff should compound
        assert!(snap.years[4].fcff > snap.years[0].fcff);
    }

    #[test]
    fn compute_dcf_rejects_terminal_growth_above_wacc() {
        let snap = compute_dcf_snapshot("X", "2026-04-14", 100.0, 40.0, 5.0, 15.0, 8.0, 20.0, 5, 10.0, 5.0, 100.0);
        assert!(!snap.note.is_empty());
        assert_eq!(snap.implied_price, 0.0);
    }

    #[test]
    fn compute_svm_triangulates_multiple_models() {
        let ddm = DdmSnapshot {
            symbol: "XYZ".into(), as_of: "2026-04-14".into(),
            annual_dividend: 3.0, implied_growth_pct: 4.0, required_return_pct: 10.0,
            growth_source: "test".into(), return_source: "test".into(),
            implied_price: 52.0, method: "Gordon Growth".into(), note: String::new(),
        };
        let dcf = DcfSnapshot {
            symbol: "XYZ".into(), as_of: "2026-04-14".into(), method: "DCF on FCFF".into(),
            base_revenue: 100.0, base_fcff: 20.0, growth_pct: 5.0, terminal_growth_pct: 2.0,
            wacc_pct: 10.0, tax_rate_pct: 20.0, fcff_margin_pct: 20.0, projection_years: 5,
            years: Vec::new(), pv_sum: 0.0, terminal_value: 0.0, pv_terminal: 0.0,
            enterprise_value: 0.0, total_debt: 0.0, cash_and_equivalents: 0.0,
            equity_value: 0.0, shares_outstanding: 1.0, implied_price: 58.0, note: String::new(),
        };
        let snap = compute_svm_snapshot(
            "XYZ", "2026-04-14", /*current*/ 50.0,
            Some(&ddm), Some(&dcf),
            Some((12.0, 4.5)),              // P/E × EPS → 54
            Some((10.0, 10.0, 5.0, 2.0, 1.0)), // EV/EBITDA 10 × 10 → EV 100 - 5 + 2 = 97 / 1 shares = 97
            Some((1.2, 45.0)),              // P/B × BVPS → 54
        );
        assert!(snap.rows.len() >= 4, "expected ≥4 triangulation rows, got {}", snap.rows.len());
        assert!(snap.fair_low > 0.0);
        assert!(snap.fair_mid >= snap.fair_low);
        assert!(snap.fair_high >= snap.fair_mid);
        assert!(snap.upside_mid_pct > 0.0, "at $50 current vs mid, upside should be positive");
    }

    #[test]
    fn compute_svm_with_no_models_emits_note() {
        let snap = compute_svm_snapshot("X", "2026-04-14", 50.0, None, None, None, None, None);
        assert!(snap.rows.is_empty());
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_ivol_rank_and_percentile() {
        let history: Vec<IvolObservation> = (0..100)
            .map(|i| IvolObservation { date: format!("2025-{:03}", i), atm_iv_pct: 20.0 + (i as f64 * 0.3) })
            .collect();
        // History spans 20% → 49.7%; current = 40%.
        let snap = compute_ivol_snapshot("TEST", "2026-04-14", 40.0, &history);
        // Rank: (40 - 20) / (49.7 - 20) × 100 ≈ 67
        assert!(snap.iv_rank > 50.0 && snap.iv_rank < 80.0,
            "expected rank 50-80, got {:.2}", snap.iv_rank);
        // Percentile: ~67% of observations ≤ 40
        assert!(snap.iv_percentile > 50.0 && snap.iv_percentile < 80.0,
            "expected percentile 50-80, got {:.2}", snap.iv_percentile);
    }

    #[test]
    fn compute_ivol_with_no_history_uses_placeholder() {
        let snap = compute_ivol_snapshot("NEW", "2026-04-14", 25.0, &[]);
        assert!(!snap.note.is_empty());
        assert!((snap.iv_52w_low_pct - 25.0).abs() < 1e-6);
        assert!((snap.iv_52w_high_pct - 25.0).abs() < 1e-6);
    }

    // ── ADR-116 Round 9 tests ──────────────────────────────────────────

    fn open_mem_conn_v9() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v9(&c).unwrap();
        c
    }

    fn synth_bars(n: usize, start: f64, daily_drift: f64) -> Vec<HistoricalPriceRow> {
        let mut bars = Vec::with_capacity(n);
        let mut px = start;
        for i in 0..n {
            let base_day = 1 + (i % 28);
            let month    = 1 + ((i / 28) % 12);
            let year     = 2024 + (i / (28 * 12));
            bars.push(HistoricalPriceRow {
                date: format!("{:04}-{:02}-{:02}", year, month, base_day),
                open: px, high: px * 1.005, low: px * 0.995,
                close: px, adj_close: px,
                volume: 1_000.0, change: 0.0, change_pct: 0.0,
            });
            px *= 1.0 + daily_drift;
        }
        bars
    }

    #[test]
    fn seasonality_snapshot_roundtrip() {
        let c = open_mem_conn_v9();
        let snap = SeasonalitySnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            years_covered: 3,
            months: vec![SeasonalityMonth {
                month: 1, label: "Jan".into(),
                avg_return_pct: 2.1, median_return_pct: 1.8, stdev_pct: 3.4,
                positive_years: 2, total_years: 3,
                best_return_pct: 5.1, worst_return_pct: -1.2,
            }],
            dow: vec![SeasonalityDow { dow: 1, label: "Mon".into(), avg_return_pct: 0.05, positive_days: 28, total_days: 52 }],
            best_month: "Jul".into(),
            worst_month: "Sep".into(),
            note: String::new(),
        };
        upsert_seasonality(&c, "AAPL", &snap).unwrap();
        let got = get_seasonality(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.months.len(), 1);
        assert_eq!(got.best_month, "Jul");
    }

    #[test]
    fn correlation_matrix_roundtrip() {
        let c = open_mem_conn_v9();
        let snap = CorrelationMatrix {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            window_days: 252,
            cells: vec![CorrelationCell { peer_symbol: "MSFT".into(), correlation: 0.85, n_observations: 245, beta_vs_peer: 0.92 }],
            mean_correlation: 0.85,
            highest_corr_symbol: "MSFT".into(),
            lowest_corr_symbol: "MSFT".into(),
            note: String::new(),
        };
        upsert_correlation(&c, "AAPL", &snap).unwrap();
        let got = get_correlation(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.cells.len(), 1);
        assert!((got.mean_correlation - 0.85).abs() < 1e-6);
    }

    #[test]
    fn total_return_snapshot_roundtrip() {
        let c = open_mem_conn_v9();
        let snap = TotalReturnSnapshot {
            symbol: "KO".into(),
            as_of: "2026-04-14".into(),
            last_close: 60.0,
            trailing_12m_dividends: 1.84,
            trailing_12m_yield_pct: 3.07,
            windows: vec![TotalReturnWindow {
                label: "1Y".into(),
                trading_days: 252,
                price_return_pct: 8.0,
                dividend_yield_pct: 3.1,
                total_return_pct: 11.1,
                annualized_pct: 11.1,
                dividends_paid: 1.84,
                n_dividends: 4,
            }],
            note: String::new(),
        };
        upsert_total_return(&c, "KO", &snap).unwrap();
        let got = get_total_return(&c, "ko").unwrap().unwrap();
        assert_eq!(got.windows.len(), 1);
        assert!((got.trailing_12m_yield_pct - 3.07).abs() < 1e-6);
    }

    #[test]
    fn technicals_snapshot_roundtrip() {
        let c = open_mem_conn_v9();
        let snap = TechnicalSnapshot {
            symbol: "NVDA".into(),
            as_of: "2026-04-14".into(),
            last_close: 850.0,
            indicators: vec![TechnicalIndicator {
                name: "RSI(14)".into(),
                value: 72.5, value_secondary: 0.0, value_tertiary: 0.0,
                signal: "overbought".into(), note: String::new(),
            }],
            trend_summary: "bullish composite".into(),
            note: String::new(),
        };
        upsert_technicals(&c, "NVDA", &snap).unwrap();
        let got = get_technicals(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.indicators.len(), 1);
        assert_eq!(got.trend_summary, "bullish composite");
    }

    #[test]
    fn vol_skew_roundtrip() {
        let c = open_mem_conn_v9();
        let snap = VolatilitySkew {
            symbol: "SPY".into(),
            as_of: "2026-04-14".into(),
            underlying_price: 520.0,
            expiries: vec![SkewExpiry {
                expiration: "2026-05-16".into(),
                days_to_expiry: 32,
                atm_iv_pct: 18.5,
                points: vec![SkewPoint {
                    strike: 520.0, moneyness_pct: 0.0,
                    call_iv_pct: 18.3, put_iv_pct: 18.7, combined_iv_pct: 18.5,
                }],
                put_call_skew_25d_pct: 2.1,
                term_note: String::new(),
            }],
            note: String::new(),
        };
        upsert_vol_skew(&c, "SPY", &snap).unwrap();
        let got = get_vol_skew(&c, "spy").unwrap().unwrap();
        assert_eq!(got.expiries.len(), 1);
        assert_eq!(got.expiries[0].points.len(), 1);
    }

    #[test]
    fn compute_seasonality_on_monthly_uptrend() {
        // 2 full years × 12 months × 21 bars = 504 bars.
        // Deterministic upward drift so every month is positive.
        let bars = synth_bars(504, 100.0, 0.001);
        let snap = compute_seasonality_snapshot("TEST", "2026-04-14", &bars);
        assert_eq!(snap.symbol, "TEST");
        assert!(snap.years_covered >= 2);
        assert!(snap.months.iter().any(|m| m.total_years > 0));
        // With uniform positive drift the best month should have a positive mean.
        let best = snap.months.iter().max_by(|a, b| a.avg_return_pct.partial_cmp(&b.avg_return_pct).unwrap()).unwrap();
        assert!(best.avg_return_pct > 0.0);
    }

    #[test]
    fn compute_seasonality_on_empty_returns_note() {
        let snap = compute_seasonality_snapshot("X", "2026-04-14", &[]);
        assert!(!snap.note.is_empty());
        assert_eq!(snap.years_covered, 0);
    }

    #[test]
    fn compute_correlation_matrix_perfect_copy() {
        // Bars need variable returns — constant drift produces zero variance
        // and an undefined ρ (our compute treats this as 0).
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        let mut px = 100.0;
        for i in 0..300 {
            let base_day = 1 + (i % 28);
            let month    = 1 + ((i / 28) % 12);
            let year     = 2024 + (i / (28 * 12));
            let drift = if i % 2 == 0 { 0.005 } else { -0.003 };
            bars.push(HistoricalPriceRow {
                date: format!("{:04}-{:02}-{:02}", year, month, base_day),
                open: px, high: px * 1.01, low: px * 0.99,
                close: px, adj_close: px,
                volume: 1_000.0, change: 0.0, change_pct: 0.0,
            });
            px *= 1.0 + drift;
        }
        let peer = bars.clone();
        let snap = compute_correlation_matrix("A", "2026-04-14", 252,
            &bars, &[("B".into(), peer)]);
        assert_eq!(snap.cells.len(), 1);
        // Perfect copy ⇒ correlation ≈ 1.0 (allow numerical slack).
        assert!(snap.cells[0].correlation > 0.999,
            "expected ρ≈1.0, got {}", snap.cells[0].correlation);
        assert!((snap.cells[0].beta_vs_peer - 1.0).abs() < 1e-6);
    }

    #[test]
    fn compute_correlation_matrix_skips_empty_peers() {
        let bars = synth_bars(300, 100.0, 0.001);
        let snap = compute_correlation_matrix("A", "2026-04-14", 252,
            &bars, &[("NO_DATA".into(), vec![])]);
        assert!(!snap.note.is_empty() || snap.cells.is_empty());
    }

    #[test]
    fn compute_total_return_with_dividends_sums_windows() {
        // synth_bars(260, ...) spans 2024-01-01 through roughly 2024-10-08, so
        // dividend ex-dates must live inside that window to be counted.
        let bars = synth_bars(260, 100.0, 0.0004);
        let divs: Vec<DividendRecord> = vec![
            DividendRecord { ex_date: "2024-03-15".into(), amount: 0.5, ..Default::default() },
            DividendRecord { ex_date: "2024-06-15".into(), amount: 0.5, ..Default::default() },
            DividendRecord { ex_date: "2024-09-15".into(), amount: 0.5, ..Default::default() },
        ];
        let snap = compute_total_return_snapshot("TEST", "2024-10-15", &bars, &divs);
        assert!(snap.windows.iter().any(|w| w.label == "1Y"));
        // At least one window should record some dividends paid.
        assert!(snap.windows.iter().any(|w| w.dividends_paid > 0.0));
    }

    #[test]
    fn compute_technical_indicators_on_uptrend_is_bullish() {
        let bars = synth_bars(120, 100.0, 0.002);
        let snap = compute_technical_indicators("TEST", "2026-04-14", &bars);
        assert!(!snap.indicators.is_empty());
        // RSI on a steady uptrend should bias above 50 (often into overbought).
        let rsi = snap.indicators.iter().find(|i| i.name.starts_with("RSI")).unwrap();
        assert!(rsi.value > 50.0, "expected RSI > 50 on uptrend, got {:.2}", rsi.value);
    }

    #[test]
    fn compute_technical_indicators_insufficient_bars_returns_note() {
        let bars = synth_bars(10, 100.0, 0.001);
        let snap = compute_technical_indicators("X", "2026-04-14", &bars);
        assert!(!snap.note.is_empty());
        assert!(snap.indicators.is_empty());
    }

    #[test]
    fn compute_volatility_skew_basic_smile() {
        let chain = OptionsChainSnapshot {
            symbol: "SPY".into(),
            as_of: "2026-04-14".into(),
            underlying_price: 500.0,
            expirations: vec![OptionExpiry {
                expiration: "2026-05-16".into(),
                days_to_expiry: 32,
                calls: vec![
                    OptionContract { strike: 450.0, option_type: "CALL".into(), implied_volatility: 0.23, in_the_money: true, ..Default::default() },
                    OptionContract { strike: 500.0, option_type: "CALL".into(), implied_volatility: 0.18, in_the_money: false, ..Default::default() },
                    OptionContract { strike: 550.0, option_type: "CALL".into(), implied_volatility: 0.21, in_the_money: false, ..Default::default() },
                ],
                puts: vec![
                    OptionContract { strike: 450.0, option_type: "PUT".into(), implied_volatility: 0.25, in_the_money: false, ..Default::default() },
                    OptionContract { strike: 500.0, option_type: "PUT".into(), implied_volatility: 0.19, in_the_money: false, ..Default::default() },
                    OptionContract { strike: 550.0, option_type: "PUT".into(), implied_volatility: 0.20, in_the_money: true, ..Default::default() },
                ],
            }],
            note: String::new(),
        };
        let snap = compute_volatility_skew("SPY", "2026-04-14", &chain);
        assert_eq!(snap.expiries.len(), 1);
        let e = &snap.expiries[0];
        assert_eq!(e.points.len(), 3);
        // ATM (500) IV should be lowest (smile).
        assert!(e.atm_iv_pct > 0.0);
        // OTM put (450) IV 25% > OTM call (550) IV 21% → positive skew.
        assert!(e.put_call_skew_25d_pct > 0.0, "expected positive skew, got {}", e.put_call_skew_25d_pct);
    }

    #[test]
    fn compute_volatility_skew_empty_chain_returns_note() {
        let chain = OptionsChainSnapshot {
            symbol: "X".into(), as_of: "2026-04-14".into(),
            underlying_price: 100.0, expirations: Vec::new(), note: String::new(),
        };
        let snap = compute_volatility_skew("X", "2026-04-14", &chain);
        assert!(!snap.note.is_empty());
    }

    // ── ADR-117 Round 10 tests ──────────────────────────────────────────

    fn open_mem_conn_v10() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v10(&c).unwrap();
        c
    }

    fn sample_statements() -> FinancialStatements {
        // Build 4 quarters of synthetic financials with positive EBITDA + FCF.
        let q = |d: &str, p: &str, ni: f64, ebitda: f64, int_exp: f64, fcf: f64| (
            IncomeStatement {
                date: d.into(), period: p.into(),
                revenue: ni * 10.0, cost_of_revenue: ni * 5.0, gross_profit: ni * 5.0,
                research_and_development: ni * 0.5, selling_general_admin: ni * 1.0,
                operating_expenses: ni * 1.5, operating_income: ni * 2.0,
                interest_expense: int_exp, ebitda, income_before_tax: ni * 1.2,
                income_tax_expense: ni * 0.2, net_income: ni, eps: ni / 1000.0,
                eps_diluted: ni / 1000.0, weighted_shares_out: 1000.0,
            },
            CashFlowStatement {
                date: d.into(), period: p.into(),
                net_income: ni, depreciation_amortization: ebitda - ni * 2.0,
                stock_based_comp: 0.0, change_working_capital: 0.0,
                cash_from_operations: fcf + 50.0, capex: -50.0,
                acquisitions: 0.0, investments_purchases: 0.0,
                cash_from_investing: -50.0, debt_repayment: 0.0,
                dividends_paid: -20.0, stock_repurchases: 0.0,
                cash_from_financing: -20.0, net_change_cash: 10.0,
                free_cash_flow: fcf,
            }
        );
        let periods = [
            ("2024-12-31", "Q4", 300.0, 500.0, 30.0, 280.0),
            ("2024-09-30", "Q3", 280.0, 480.0, 30.0, 260.0),
            ("2024-06-30", "Q2", 260.0, 460.0, 30.0, 240.0),
            ("2024-03-31", "Q1", 240.0, 440.0, 30.0, 220.0),
        ];
        let mut income_q = Vec::new();
        let mut cf_q = Vec::new();
        for (d, p, ni, ebitda, int_exp, fcf) in periods.iter() {
            let (i, c) = q(d, p, *ni, *ebitda, *int_exp, *fcf);
            income_q.push(i);
            cf_q.push(c);
        }
        let bal = BalanceSheet {
            date: "2024-12-31".into(), period: "FY".into(),
            cash_and_equiv: 500.0, short_term_investments: 0.0,
            net_receivables: 100.0, inventory: 200.0,
            total_current_assets: 800.0,
            property_plant_equipment: 1000.0, goodwill: 0.0, intangible_assets: 0.0,
            long_term_investments: 0.0, total_non_current_assets: 1000.0,
            total_assets: 1800.0,
            accounts_payable: 150.0, short_term_debt: 100.0,
            total_current_liabilities: 400.0, long_term_debt: 600.0,
            total_non_current_liabilities: 800.0, total_liabilities: 1200.0,
            common_stock: 200.0, retained_earnings: 400.0, total_equity: 600.0,
            total_debt: 700.0, net_debt: 200.0,
        };
        FinancialStatements {
            income_annual: vec![income_q[0].clone()],
            income_quarterly: income_q,
            balance_annual: vec![bal.clone()],
            balance_quarterly: vec![bal],
            cashflow_annual: vec![cf_q[0].clone()],
            cashflow_quarterly: cf_q,
        }
    }

    #[test]
    fn leverage_snapshot_roundtrip() {
        let c = open_mem_conn_v10();
        let snap = LeverageSnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            total_debt: 700.0, net_debt: 200.0,
            ebitda_ttm: 1880.0, interest_expense_ttm: 120.0,
            total_equity: 600.0,
            ratios: vec![LeverageRatio {
                name: "Debt / EBITDA".into(), value: 0.37,
                peer_median: 0.0, signal: "HEALTHY".into(),
                note: "".into(),
            }],
            solvency_summary: "HEALTHY".into(),
            note: "".into(),
        };
        upsert_leverage(&c, "AAPL", &snap).unwrap();
        let got = get_leverage(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.ratios.len(), 1);
        assert_eq!(got.solvency_summary, "HEALTHY");
    }

    #[test]
    fn accruals_snapshot_roundtrip() {
        let c = open_mem_conn_v10();
        let snap = AccrualsSnapshot {
            symbol: "MSFT".into(), as_of: "2026-04-14".into(),
            ttm_net_income: 1000.0, ttm_free_cash_flow: 900.0,
            ttm_cash_conversion_pct: 90.0, avg_cash_conversion_pct: 85.0,
            periods: vec![], trend_label: "STABLE".into(), note: "".into(),
        };
        upsert_accruals(&c, "MSFT", &snap).unwrap();
        let got = get_accruals(&c, "msft").unwrap().unwrap();
        assert_eq!(got.ttm_cash_conversion_pct, 90.0);
    }

    #[test]
    fn realized_vol_snapshot_roundtrip() {
        let c = open_mem_conn_v10();
        let snap = RealizedVolSnapshot {
            symbol: "NVDA".into(), as_of: "2026-04-14".into(),
            last_close: 900.0, current_atm_iv_pct: 45.0,
            iv_rv_gap_pct: 10.0, iv_rv_ratio: 1.28,
            windows: vec![RealizedVolWindow {
                label: "20d".into(), trading_days: 20,
                realized_vol_pct: 35.0, percentile: 60.0, n_observations: 100,
            }],
            regime_label: "RICH_IV".into(), note: "".into(),
        };
        upsert_realized_vol(&c, "NVDA", &snap).unwrap();
        let got = get_realized_vol(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.windows.len(), 1);
        assert_eq!(got.regime_label, "RICH_IV");
    }

    #[test]
    fn fcf_yield_snapshot_roundtrip() {
        let c = open_mem_conn_v10();
        let snap = FcfYieldSnapshot {
            symbol: "KO".into(), as_of: "2026-04-14".into(),
            market_cap: 300_000_000_000.0, ttm_free_cash_flow: 10_000_000_000.0,
            ttm_dividends_paid: 8_000_000_000.0, ttm_fcf_yield_pct: 3.33,
            ttm_dividend_yield_pct: 2.67, ttm_payout_from_fcf_pct: 80.0,
            ttm_payout_from_ni_pct: 70.0, fcf_cagr_5y_pct: 5.2,
            periods: vec![], sustainability_label: "STRETCHED".into(),
            note: "".into(),
        };
        upsert_fcf_yield(&c, "KO", &snap).unwrap();
        let got = get_fcf_yield(&c, "ko").unwrap().unwrap();
        assert_eq!(got.sustainability_label, "STRETCHED");
    }

    #[test]
    fn short_interest_snapshot_roundtrip() {
        let c = open_mem_conn_v10();
        let snap = ShortInterestSnapshot {
            symbol: "GME".into(), as_of: "2026-04-14".into(),
            shares_outstanding: 300_000_000.0, shares_float: 200_000_000.0,
            short_shares: 50_000_000.0, short_percent_of_float: 25.0,
            avg_daily_volume_20d: 5_000_000.0, days_to_cover: 10.0,
            short_ratio_reported: 0.0, utilization_proxy_pct: 25.0,
            squeeze_risk_label: "EXTREME".into(), note: "".into(),
        };
        upsert_short_interest(&c, "GME", &snap).unwrap();
        let got = get_short_interest(&c, "gme").unwrap().unwrap();
        assert_eq!(got.squeeze_risk_label, "EXTREME");
        assert_eq!(got.days_to_cover, 10.0);
    }

    #[test]
    fn compute_leverage_on_healthy_statements() {
        let st = sample_statements();
        let snap = compute_leverage_snapshot("TEST", "2026-04-14", &st, 700.0, 500.0);
        // TTM EBITDA = 500+480+460+440 = 1880 → Debt/EBITDA = 700/1880 ≈ 0.37.
        assert!(!snap.ratios.is_empty());
        let de = snap.ratios.iter().find(|r| r.name == "Debt / EBITDA").unwrap();
        assert!((de.value - 700.0 / 1880.0).abs() < 1e-6);
        assert_eq!(de.signal, "HEALTHY");
        assert_eq!(snap.ebitda_ttm, 1880.0);
    }

    #[test]
    fn compute_leverage_empty_statements_produces_note() {
        let st = FinancialStatements::default();
        let snap = compute_leverage_snapshot("X", "2026-04-14", &st, 0.0, 0.0);
        assert!(snap.ratios.is_empty());
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_accruals_high_conversion_labels_high() {
        let st = sample_statements();
        let snap = compute_accruals_snapshot("TEST", "2026-04-14", &st);
        // Q4: NI=300, FCF=280 → conv=93.3% → HIGH.
        assert_eq!(snap.periods.len(), 4);
        let latest = &snap.periods[0];
        assert_eq!(latest.quality_label, "HIGH");
        let ttm_ni: f64 = 300.0 + 280.0 + 260.0 + 240.0;
        assert!((snap.ttm_net_income - ttm_ni).abs() < 1e-6);
    }

    #[test]
    fn compute_accruals_insufficient_periods_labels_insufficient() {
        let mut st = FinancialStatements::default();
        st.income_quarterly.push(IncomeStatement {
            date: "2024-12-31".into(), period: "Q4".into(),
            net_income: 100.0, ..Default::default()
        });
        st.cashflow_quarterly.push(CashFlowStatement {
            date: "2024-12-31".into(), period: "Q4".into(),
            net_income: 100.0, free_cash_flow: 90.0,
            ..Default::default()
        });
        let snap = compute_accruals_snapshot("X", "2026-04-14", &st);
        assert_eq!(snap.periods.len(), 1);
        assert_eq!(snap.trend_label, "INSUFFICIENT");
    }

    #[test]
    fn compute_realized_vol_with_drift_produces_rich_regime() {
        let bars = synth_bars(260, 100.0, 0.001);
        let snap = compute_realized_vol_snapshot("TEST", "2026-04-14", &bars, 40.0);
        assert!(!snap.windows.is_empty());
        assert!(snap.windows.iter().any(|w| w.label == "20d"));
        // Constant drift → near-zero RV → IV/RV should flag RICH_IV or NO_IV_REFERENCE.
        assert!(snap.regime_label == "RICH_IV" || snap.regime_label == "NO_IV_REFERENCE");
    }

    #[test]
    fn compute_realized_vol_insufficient_bars_returns_note() {
        let bars = synth_bars(10, 100.0, 0.001);
        let snap = compute_realized_vol_snapshot("X", "2026-04-14", &bars, 40.0);
        assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_fcf_yield_with_market_cap() {
        let st = sample_statements();
        let snap = compute_fcf_yield_snapshot("TEST", "2026-04-14", &st, 100_000.0, 100.0);
        // TTM FCF = 280+260+240+220 = 1000; yield = 1000/100000 = 1.0%
        assert!((snap.ttm_free_cash_flow - 1000.0).abs() < 1e-6);
        assert!((snap.ttm_fcf_yield_pct - 1.0).abs() < 1e-6);
        // TTM dividends paid = 20*4 = 80, payout_fcf = 80/1000 = 8%, label SAFE.
        assert_eq!(snap.sustainability_label, "SAFE");
    }

    #[test]
    fn compute_fcf_yield_no_market_cap_emits_note() {
        let st = sample_statements();
        let snap = compute_fcf_yield_snapshot("X", "2026-04-14", &st, 0.0, 100.0);
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_short_interest_high_risk_squeeze() {
        let bars = synth_bars(30, 100.0, 0.0);
        // 200M float × 25% = 50M short; 50M / 1K avg = 50K days-to-cover → EXTREME.
        let snap = compute_short_interest_snapshot(
            "GME", "2026-04-14",
            300_000_000.0, 200_000_000.0, 25.0, 0.0, &bars,
        );
        assert_eq!(snap.short_shares, 50_000_000.0);
        assert_eq!(snap.squeeze_risk_label, "EXTREME");
    }

    #[test]
    fn compute_short_interest_no_shorts_insufficient() {
        let bars = synth_bars(30, 100.0, 0.0);
        let snap = compute_short_interest_snapshot(
            "X", "2026-04-14",
            100_000_000.0, 80_000_000.0, 0.0, 0.0, &bars,
        );
        assert_eq!(snap.squeeze_risk_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-118 Godel Parity Round 11 tests ─────────────────────────────

    fn open_mem_conn_v11() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v11(&c).unwrap();
        c
    }

    fn two_year_statements() -> FinancialStatements {
        // Build two annual snapshots so Piotroski and Altman have enough data.
        let inc_current = IncomeStatement {
            date: "2024-12-31".into(), period: "FY".into(),
            revenue: 2000.0, cost_of_revenue: 1000.0, gross_profit: 1000.0,
            research_and_development: 100.0, selling_general_admin: 200.0,
            operating_expenses: 300.0, operating_income: 700.0,
            interest_expense: 50.0, ebitda: 800.0,
            income_before_tax: 650.0, income_tax_expense: 150.0,
            net_income: 500.0, eps: 5.0, eps_diluted: 5.0,
            weighted_shares_out: 100.0,
        };
        let inc_prior = IncomeStatement {
            date: "2023-12-31".into(), period: "FY".into(),
            revenue: 1800.0, cost_of_revenue: 1000.0, gross_profit: 800.0,
            research_and_development: 100.0, selling_general_admin: 200.0,
            operating_expenses: 300.0, operating_income: 500.0,
            interest_expense: 50.0, ebitda: 600.0,
            income_before_tax: 450.0, income_tax_expense: 100.0,
            net_income: 350.0, eps: 3.5, eps_diluted: 3.5,
            weighted_shares_out: 100.0,
        };
        let bal_current = BalanceSheet {
            date: "2024-12-31".into(), period: "FY".into(),
            cash_and_equiv: 400.0, short_term_investments: 0.0,
            net_receivables: 200.0, inventory: 300.0,
            total_current_assets: 900.0,
            property_plant_equipment: 1500.0, goodwill: 0.0, intangible_assets: 0.0,
            long_term_investments: 0.0, total_non_current_assets: 1500.0,
            total_assets: 2400.0,
            accounts_payable: 150.0, short_term_debt: 100.0,
            total_current_liabilities: 400.0, long_term_debt: 500.0,
            total_non_current_liabilities: 700.0, total_liabilities: 1100.0,
            common_stock: 300.0, retained_earnings: 1000.0, total_equity: 1300.0,
            total_debt: 600.0, net_debt: 200.0,
        };
        let bal_prior = BalanceSheet {
            date: "2023-12-31".into(), period: "FY".into(),
            cash_and_equiv: 300.0, short_term_investments: 0.0,
            net_receivables: 180.0, inventory: 280.0,
            total_current_assets: 760.0,
            property_plant_equipment: 1400.0, goodwill: 0.0, intangible_assets: 0.0,
            long_term_investments: 0.0, total_non_current_assets: 1400.0,
            total_assets: 2160.0,
            accounts_payable: 150.0, short_term_debt: 100.0,
            total_current_liabilities: 400.0, long_term_debt: 600.0,
            total_non_current_liabilities: 800.0, total_liabilities: 1200.0,
            common_stock: 300.0, retained_earnings: 660.0, total_equity: 960.0,
            total_debt: 700.0, net_debt: 400.0,
        };
        let cf_current = CashFlowStatement {
            date: "2024-12-31".into(), period: "FY".into(),
            net_income: 500.0, depreciation_amortization: 100.0,
            stock_based_comp: 0.0, change_working_capital: 0.0,
            cash_from_operations: 600.0, capex: -200.0,
            acquisitions: 0.0, investments_purchases: 0.0,
            cash_from_investing: -200.0, debt_repayment: -100.0,
            dividends_paid: 0.0, stock_repurchases: 0.0,
            cash_from_financing: -100.0, net_change_cash: 300.0,
            free_cash_flow: 400.0,
        };
        let cf_prior = CashFlowStatement {
            date: "2023-12-31".into(), period: "FY".into(),
            net_income: 350.0, depreciation_amortization: 90.0,
            stock_based_comp: 0.0, change_working_capital: 0.0,
            cash_from_operations: 420.0, capex: -180.0,
            acquisitions: 0.0, investments_purchases: 0.0,
            cash_from_investing: -180.0, debt_repayment: 0.0,
            dividends_paid: 0.0, stock_repurchases: 0.0,
            cash_from_financing: 0.0, net_change_cash: 240.0,
            free_cash_flow: 240.0,
        };
        FinancialStatements {
            income_annual: vec![inc_current.clone(), inc_prior.clone()],
            income_quarterly: vec![inc_current],
            balance_annual: vec![bal_current.clone(), bal_prior.clone()],
            balance_quarterly: vec![bal_current],
            cashflow_annual: vec![cf_current.clone(), cf_prior.clone()],
            cashflow_quarterly: vec![cf_current],
        }
    }

    #[test]
    fn altman_z_snapshot_roundtrip() {
        let c = open_mem_conn_v11();
        let snap = AltmanZSnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            working_capital: 500.0, retained_earnings: 1000.0,
            ebit: 700.0, market_value_equity: 5000.0,
            sales: 2000.0, total_assets: 2400.0, total_liabilities: 1100.0,
            z_score: 4.2, zone: "SAFE".into(),
            components: vec![AltmanComponent {
                name: "A: WC/TA".into(), ratio: 0.208, coefficient: 1.2,
                contribution: 0.25, note: "".into(),
            }],
            note: "".into(),
        };
        upsert_altman_z(&c, "AAPL", &snap).unwrap();
        let got = get_altman_z(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.symbol, "AAPL");
        assert_eq!(got.zone, "SAFE");
        assert_eq!(got.components.len(), 1);
    }

    #[test]
    fn piotroski_snapshot_roundtrip() {
        let c = open_mem_conn_v11();
        let snap = PiotroskiSnapshot {
            symbol: "MSFT".into(), as_of: "2026-04-14".into(),
            current_period: "2024-12-31".into(), prior_period: "2023-12-31".into(),
            f_score: 8, strength_label: "STRONG".into(),
            profitability_score: 4, leverage_score: 2, efficiency_score: 2,
            checks: vec![PiotroskiCheck {
                category: "Profitability".into(),
                name: "Positive Net Income".into(),
                passed: true, value_current: 500.0, value_prior: 350.0,
                note: "".into(),
            }],
            note: "".into(),
        };
        upsert_piotroski(&c, "MSFT", &snap).unwrap();
        let got = get_piotroski(&c, "msft").unwrap().unwrap();
        assert_eq!(got.f_score, 8);
        assert_eq!(got.strength_label, "STRONG");
    }

    #[test]
    fn ohlc_vol_snapshot_roundtrip() {
        let c = open_mem_conn_v11();
        let snap = OhlcVolSnapshot {
            symbol: "NVDA".into(), as_of: "2026-04-14".into(),
            trading_days: 60,
            estimators: vec![VolEstimator {
                name: "YangZhang".into(), annualized_vol_pct: 35.0,
                efficiency_vs_close: 1.1, note: "".into(),
            }],
            preferred_estimate_pct: 35.0, preferred_label: "YangZhang".into(),
            note: "".into(),
        };
        upsert_ohlc_vol(&c, "NVDA", &snap).unwrap();
        let got = get_ohlc_vol(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.preferred_label, "YangZhang");
        assert_eq!(got.estimators.len(), 1);
    }

    #[test]
    fn eps_beat_snapshot_roundtrip() {
        let c = open_mem_conn_v11();
        let snap = EpsBeatSnapshot {
            symbol: "AMZN".into(), as_of: "2026-04-14".into(),
            total_reports: 8, beats: 6, misses: 1, inlines: 1,
            beat_rate_pct: 75.0, current_streak: 3,
            longest_beat_streak: 4, longest_miss_streak: 1,
            avg_surprise_pct: 5.2, median_surprise_pct: 4.8,
            recent_avg_surprise_pct: 6.5,
            bias_label: "POSITIVE".into(), trend_label: "ACCELERATING".into(),
            latest_date: "2024-10-31".into(), latest_surprise_pct: 7.0,
            note: "".into(),
        };
        upsert_eps_beat(&c, "AMZN", &snap).unwrap();
        let got = get_eps_beat(&c, "amzn").unwrap().unwrap();
        assert_eq!(got.beats, 6);
        assert_eq!(got.trend_label, "ACCELERATING");
    }

    #[test]
    fn price_target_dispersion_roundtrip() {
        let c = open_mem_conn_v11();
        let snap = PriceTargetDispersion {
            symbol: "TSLA".into(), as_of: "2026-04-14".into(),
            current_price: 200.0,
            target_high: 350.0, target_low: 150.0,
            target_mean: 250.0, target_median: 240.0,
            num_analysts: 25,
            dispersion_pct: 80.0, spread_pct: 100.0,
            implied_return_median_pct: 20.0, implied_return_mean_pct: 25.0,
            upside_to_high_pct: 75.0, downside_to_low_pct: -25.0,
            consensus_label: "BULLISH".into(), note: "".into(),
        };
        upsert_price_target_dispersion(&c, "TSLA", &snap).unwrap();
        let got = get_price_target_dispersion(&c, "tsla").unwrap().unwrap();
        assert_eq!(got.consensus_label, "BULLISH");
        assert_eq!(got.num_analysts, 25);
    }

    #[test]
    fn compute_altman_z_on_healthy_statements() {
        let st = two_year_statements();
        let snap = compute_altman_z_snapshot("TEST", "2026-04-14", &st, 5000.0);
        // WC = 900-400 = 500, TA = 2400, RE = 1000, EBIT = 700, MVE = 5000, TL = 1100, Sales = 2000
        // Z = 1.2*(500/2400) + 1.4*(1000/2400) + 3.3*(700/2400) + 0.6*(5000/1100) + 1.0*(2000/2400)
        //   ≈ 0.25 + 0.583 + 0.963 + 2.727 + 0.833 ≈ 5.36 → SAFE
        assert_eq!(snap.components.len(), 5);
        assert!(snap.z_score > 2.99);
        assert_eq!(snap.zone, "SAFE");
        assert_eq!(snap.total_assets, 2400.0);
    }

    #[test]
    fn compute_altman_z_insufficient_data_returns_note() {
        let st = FinancialStatements::default();
        let snap = compute_altman_z_snapshot("X", "2026-04-14", &st, 1000.0);
        assert_eq!(snap.zone, "INSUFFICIENT_DATA");
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_piotroski_strong_score() {
        let st = two_year_statements();
        let snap = compute_piotroski_snapshot("TEST", "2026-04-14", &st);
        // Improving NI (350→500), positive OCF (600), OCF>NI, LTDebt↓ (600→500),
        // current ratio ≈ 900/400 vs 760/400 → improved, no new shares, GM ≈ 50% vs 44% → improved,
        // asset turnover ≈ 2000/2400 vs 1800/2160 → similar, expect STRONG (≥7).
        assert!(snap.f_score >= 7);
        assert_eq!(snap.strength_label, "STRONG");
        assert_eq!(snap.checks.len(), 9);
    }

    #[test]
    fn compute_piotroski_insufficient_data() {
        let st = FinancialStatements::default();
        let snap = compute_piotroski_snapshot("X", "2026-04-14", &st);
        assert_eq!(snap.strength_label, "INSUFFICIENT_DATA");
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_ohlc_vol_five_estimators() {
        let bars = synth_bars(60, 100.0, 0.001);
        let snap = compute_ohlc_vol_snapshot("TEST", "2026-04-14", &bars, 30);
        assert_eq!(snap.estimators.len(), 5);
        assert!(snap.preferred_estimate_pct >= 0.0);
        assert_eq!(snap.preferred_label, "Yang-Zhang");
    }

    #[test]
    fn compute_ohlc_vol_insufficient_bars() {
        let bars = synth_bars(10, 100.0, 0.001);
        let snap = compute_ohlc_vol_snapshot("X", "2026-04-14", &bars, 20);
        assert!(!snap.note.is_empty());
        assert!(snap.estimators.is_empty());
    }

    #[test]
    fn compute_eps_beat_six_beats_labels_positive() {
        let rows: Vec<EarningsSurprise> = (0..8).map(|i| EarningsSurprise {
            date: format!("2024-{:02}-01", i + 1),
            symbol: "TEST".into(),
            eps_actual: 1.0 + (i as f64) * 0.05,
            eps_estimate: 1.0,
            surprise: (i as f64) * 0.05,
            surprise_pct: (i as f64) * 5.0,
        }).collect();
        let snap = compute_eps_beat_snapshot("TEST", "2026-04-14", &rows);
        assert_eq!(snap.total_reports, 8);
        assert!(snap.beats >= 6);
        assert_eq!(snap.bias_label, "POSITIVE");
        assert!(snap.current_streak > 0);
    }

    #[test]
    fn compute_eps_beat_empty_reports() {
        let snap = compute_eps_beat_snapshot("X", "2026-04-14", &[]);
        assert_eq!(snap.total_reports, 0);
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn compute_price_target_dispersion_bullish() {
        let target = PriceTarget {
            symbol: "TEST".into(),
            target_high: 150.0, target_low: 110.0,
            target_mean: 130.0, target_median: 125.0,
            last_updated: "2024-11-01".into(),
            num_analysts: 15,
        };
        let snap = compute_price_target_dispersion("TEST", "2026-04-14", 100.0, Some(&target));
        // implied_median = (125-100)/100 = 25% → BULLISH
        assert_eq!(snap.consensus_label, "BULLISH");
        assert!((snap.implied_return_median_pct - 25.0).abs() < 1e-6);
        assert!((snap.spread_pct - 40.0).abs() < 1e-6);
    }

    #[test]
    fn compute_price_target_dispersion_no_coverage() {
        let snap = compute_price_target_dispersion("X", "2026-04-14", 100.0, None);
        assert_eq!(snap.consensus_label, "NO_COVERAGE");
        assert!(!snap.note.is_empty());
    }

    // ── ADR-119 Godel Parity Round 12 tests ────────────────────────────────

    fn open_mem_conn_v12() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v12(&c).unwrap();
        c
    }

    fn trade(date: &str, name: &str, ttype: &str, disp: &str, shares: f64, price: f64) -> InsiderTrade {
        InsiderTrade {
            filing_date: date.to_string(),
            transaction_date: date.to_string(),
            reporting_name: name.to_string(),
            transaction_type: ttype.to_string(),
            acquisition_disposition: disp.to_string(),
            shares,
            price,
            value_usd: shares * price,
            shares_owned_after: 0.0,
            link: String::new(),
        }
    }

    #[test]
    fn insider_activity_snapshot_roundtrip() {
        let c = open_mem_conn_v12();
        let snap = InsiderActivitySnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            window_days: 90,
            total_trades: 5,
            buy_count: 3,
            sell_count: 2,
            other_count: 0,
            unique_insiders: 2,
            gross_buy_value_usd: 1_000_000.0,
            gross_sell_value_usd: 400_000.0,
            net_value_usd: 600_000.0,
            buy_sell_ratio: 1.5,
            net_shares: 5_000.0,
            latest_trade_date: "2026-04-10".into(),
            bias_label: "BULLISH".into(),
            conviction_label: "MEDIUM".into(),
            note: String::new(),
        };
        upsert_insider_activity(&c, "AAPL", &snap).unwrap();
        let got = get_insider_activity(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.bias_label, "BULLISH");
        assert_eq!(got.buy_count, 3);
        assert!((got.buy_sell_ratio - 1.5).abs() < 1e-6);
    }

    #[test]
    fn compute_insider_activity_bullish_net_buys() {
        let trades = vec![
            trade("2026-04-10", "Alice CEO", "P-Purchase", "A", 1000.0, 150.0),
            trade("2026-04-05", "Bob CFO",   "P-Purchase", "A", 2000.0, 148.0),
            trade("2026-03-20", "Alice CEO", "P-Purchase", "A",  500.0, 145.0),
            trade("2026-03-10", "Carol COO", "S-Sale",     "D", 500.0, 160.0),
        ];
        let snap = compute_insider_activity_snapshot("AAPL", "2026-04-14", &trades, 90);
        assert_eq!(snap.bias_label, "BULLISH");
        assert_eq!(snap.buy_count, 3);
        assert_eq!(snap.sell_count, 1);
        assert_eq!(snap.unique_insiders, 3);
        assert!(snap.gross_buy_value_usd > snap.gross_sell_value_usd);
        assert!(snap.net_value_usd > 0.0);
        assert_eq!(snap.latest_trade_date, "2026-04-10");
    }

    #[test]
    fn compute_insider_activity_bearish_net_sales() {
        let trades = vec![
            trade("2026-04-10", "Alice CEO", "S-Sale", "D", 10_000.0, 150.0),
            trade("2026-04-05", "Bob CFO",   "S-Sale", "D",  5_000.0, 148.0),
            trade("2026-03-10", "Alice CEO", "P-Purchase", "A", 100.0, 145.0),
        ];
        let snap = compute_insider_activity_snapshot("AAPL", "2026-04-14", &trades, 90);
        assert_eq!(snap.bias_label, "BEARISH");
        assert_eq!(snap.sell_count, 2);
        assert!(snap.net_value_usd < 0.0);
    }

    #[test]
    fn compute_insider_activity_no_activity() {
        let snap = compute_insider_activity_snapshot("X", "2026-04-14", &[], 90);
        assert_eq!(snap.bias_label, "NO_ACTIVITY");
        assert_eq!(snap.conviction_label, "NONE");
        assert!(!snap.note.is_empty());
    }

    #[test]
    fn insider_activity_respects_lookback_window() {
        let trades = vec![
            trade("2026-04-01", "Alice CEO", "P-Purchase", "A", 1000.0, 150.0),
            trade("2025-01-01", "Old CEO",   "P-Purchase", "A", 9999.0, 100.0),
        ];
        // 90-day window from 2026-04-14 should exclude 2025-01-01
        let snap = compute_insider_activity_snapshot("X", "2026-04-14", &trades, 90);
        assert_eq!(snap.total_trades, 1);
        assert_eq!(snap.buy_count, 1);
    }

    fn dvd(ex: &str, amt: f64) -> DividendRecord {
        DividendRecord { ex_date: ex.to_string(), amount: amt, ..Default::default() }
    }

    #[test]
    fn divg_snapshot_roundtrip() {
        let c = open_mem_conn_v12();
        let snap = DivgSnapshot {
            symbol: "KO".into(),
            as_of: "2026-04-14".into(),
            total_payments: 20,
            latest_amount: 0.49,
            annualized_dividend: 1.96,
            years_covered: 5,
            cagr_1y_pct: 5.0,
            cagr_3y_pct: 4.5,
            cagr_5y_pct: 4.0,
            trend_label: "GROWING".into(),
            ..Default::default()
        };
        upsert_divg(&c, "KO", &snap).unwrap();
        let got = get_divg(&c, "ko").unwrap().unwrap();
        assert_eq!(got.trend_label, "GROWING");
        assert!((got.cagr_1y_pct - 5.0).abs() < 1e-6);
    }

    #[test]
    fn compute_divg_growing_consistent() {
        // Five complete years of 4 payments each, 5 % YoY growth, last payment before as_of.
        let mut rows = Vec::new();
        for (y, base) in [(2020, 0.40), (2021, 0.42), (2022, 0.44), (2023, 0.46), (2024, 0.49)] {
            for q in ["03-15", "06-15", "09-15", "12-15"] {
                rows.push(dvd(&format!("{y}-{q}"), base));
            }
        }
        let snap = compute_divg_snapshot("KO", "2026-04-14", &rows);
        assert_eq!(snap.trend_label, "GROWING");
        assert!(snap.years_covered >= 5);
        assert!(snap.cagr_1y_pct > 0.0);
        assert!(snap.consecutive_growth_years >= 4);
        assert!(snap.consistency_score_pct >= 70.0);
    }

    #[test]
    fn compute_divg_cutting() {
        let rows = vec![
            dvd("2022-03-15", 0.80),
            dvd("2022-06-15", 0.80),
            dvd("2022-09-15", 0.80),
            dvd("2022-12-15", 0.80),
            dvd("2023-03-15", 0.70),
            dvd("2023-06-15", 0.60),
            dvd("2023-09-15", 0.50),
            dvd("2023-12-15", 0.40),
        ];
        let snap = compute_divg_snapshot("X", "2026-04-14", &rows);
        assert_eq!(snap.trend_label, "CUTTING");
        assert!(snap.cagr_1y_pct < 0.0);
    }

    #[test]
    fn compute_divg_no_history() {
        let snap = compute_divg_snapshot("X", "2026-04-14", &[]);
        assert_eq!(snap.trend_label, "NO_HISTORY");
    }

    fn inc(date: &str, rev: f64, eps: f64) -> IncomeStatement {
        IncomeStatement {
            date: date.to_string(),
            period: "Q".to_string(),
            revenue: rev,
            eps,
            ..Default::default()
        }
    }

    fn surp(date: &str, actual: f64, est: f64) -> EarningsSurprise {
        let s = actual - est;
        EarningsSurprise {
            date: date.to_string(),
            symbol: "X".into(),
            eps_actual: actual,
            eps_estimate: est,
            surprise: s,
            surprise_pct: if est.abs() > 0.0 { s / est.abs() * 100.0 } else { 0.0 },
        }
    }

    #[test]
    fn earm_snapshot_roundtrip() {
        let c = open_mem_conn_v12();
        let snap = EarmSnapshot {
            symbol: "NVDA".into(),
            as_of: "2026-04-14".into(),
            quarters_used: 8,
            recent_revenue_growth_pct: 40.0,
            prior_revenue_growth_pct: 25.0,
            revenue_acceleration_pct: 15.0,
            recent_eps_surprise_pct: 10.0,
            prior_eps_surprise_pct: 5.0,
            eps_surprise_acceleration_pct: 5.0,
            composite_score: 80.0,
            momentum_label: "ACCELERATING".into(),
            quarters: vec![EarmQuarterRow {
                period: "2026-01-31".into(),
                revenue: 22000.0,
                revenue_yoy_pct: 40.0,
                eps_actual: 5.0,
                eps_estimate: 4.5,
                eps_surprise_pct: 11.1,
            }],
            note: String::new(),
        };
        upsert_earm(&c, "NVDA", &snap).unwrap();
        let got = get_earm(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.momentum_label, "ACCELERATING");
        assert!((got.composite_score - 80.0).abs() < 1e-6);
    }

    #[test]
    fn compute_earm_accelerating() {
        // 8 quarters, newest-first. Revenue grows yoy, recent pace faster than prior.
        let statements = FinancialStatements {
            income_quarterly: vec![
                inc("2026-03-31", 140.0, 2.40),   // 0  yoy vs 4 = +16.67%
                inc("2025-12-31", 135.0, 2.30),   // 1  yoy vs 5 = +17.39%
                inc("2025-09-30", 130.0, 2.20),   // 2  yoy vs 6 = +18.18%
                inc("2025-06-30", 125.0, 2.10),   // 3  yoy vs 7 = +19.05%
                inc("2025-03-31", 120.0, 2.00),   // 4
                inc("2024-12-31", 115.0, 1.90),   // 5
                inc("2024-09-30", 110.0, 1.80),   // 6
                inc("2024-06-30", 105.0, 1.75),   // 7
            ],
            ..Default::default()
        };
        let surprises = vec![
            surp("2026-03-31", 2.40, 2.30),
            surp("2025-12-31", 2.30, 2.20),
            surp("2025-09-30", 2.20, 2.15),
            surp("2025-06-30", 2.10, 2.08),
            surp("2025-03-31", 2.00, 1.99),
            surp("2024-12-31", 1.90, 1.90),
            surp("2024-09-30", 1.80, 1.81),
            surp("2024-06-30", 1.75, 1.77),
        ];
        let snap = compute_earm_snapshot("NVDA", "2026-04-14", &statements, &surprises);
        assert_eq!(snap.quarters_used, 8);
        assert!(snap.recent_revenue_growth_pct > 0.0);
        assert!(snap.composite_score > 0.0);
        assert!(matches!(snap.momentum_label.as_str(), "ACCELERATING" | "STABLE"));
    }

    #[test]
    fn compute_earm_insufficient_data() {
        let statements = FinancialStatements {
            income_quarterly: vec![inc("2026-03-31", 100.0, 1.0)],
            ..Default::default()
        };
        let snap = compute_earm_snapshot("X", "2026-04-14", &statements, &[]);
        assert_eq!(snap.momentum_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn sector_rotation_snapshot_roundtrip() {
        let c = open_mem_conn_v12();
        let snap = SectorRotationSnapshot {
            symbol: "AAPL".into(),
            as_of: "2026-04-14".into(),
            symbol_sector: "Technology".into(),
            symbol_sector_change_pct: 1.5,
            sector_rank: 1,
            sectors_total: 11,
            avg_sector_change_pct: 0.3,
            relative_strength_pct: 1.2,
            strength_label: "LEADER".into(),
            ..Default::default()
        };
        upsert_sector_rotation(&c, "AAPL", &snap).unwrap();
        let got = get_sector_rotation(&c, "aapl").unwrap().unwrap();
        assert_eq!(got.strength_label, "LEADER");
        assert_eq!(got.sector_rank, 1);
    }

    #[test]
    fn compute_sector_rotation_leader() {
        let sectors = vec![
            SectorPerformance { sector: "Technology".into(),          change_pct: 2.0 },
            SectorPerformance { sector: "Healthcare".into(),          change_pct: 0.5 },
            SectorPerformance { sector: "Financial Services".into(),  change_pct: 0.1 },
            SectorPerformance { sector: "Energy".into(),              change_pct: -0.5 },
            SectorPerformance { sector: "Consumer Cyclical".into(),   change_pct: 1.0 },
            SectorPerformance { sector: "Utilities".into(),           change_pct: -1.0 },
        ];
        let snap = compute_sector_rotation_snapshot("AAPL", "2026-04-14", "Technology", &sectors);
        assert_eq!(snap.strength_label, "LEADER");
        assert_eq!(snap.sector_rank, 1);
        assert_eq!(snap.strongest_sector, "Technology");
        assert_eq!(snap.weakest_sector, "Utilities");
        assert!(snap.relative_strength_pct > 0.0);
    }

    #[test]
    fn compute_sector_rotation_laggard() {
        let sectors = vec![
            SectorPerformance { sector: "Technology".into(),  change_pct: 2.0 },
            SectorPerformance { sector: "Healthcare".into(),  change_pct: 1.5 },
            SectorPerformance { sector: "Financials".into(),  change_pct: 1.0 },
            SectorPerformance { sector: "Energy".into(),      change_pct: -1.5 },
        ];
        let snap = compute_sector_rotation_snapshot("XOM", "2026-04-14", "Energy", &sectors);
        assert_eq!(snap.strength_label, "LAGGARD");
        assert!(snap.relative_strength_pct < 0.0);
    }

    #[test]
    fn compute_sector_rotation_no_data() {
        let snap = compute_sector_rotation_snapshot("X", "2026-04-14", "Technology", &[]);
        assert_eq!(snap.strength_label, "NO_DATA");
    }

    fn rc(date: &str, action: &str, firm: &str, to: &str) -> RatingChange {
        RatingChange {
            date: date.to_string(),
            symbol: "X".into(),
            company: String::new(),
            firm: firm.to_string(),
            action: action.to_string(),
            from_grade: String::new(),
            to_grade: to.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn updm_snapshot_roundtrip() {
        let c = open_mem_conn_v12();
        let snap = UpdmSnapshot {
            symbol: "NVDA".into(),
            as_of: "2026-04-14".into(),
            total_actions: 8,
            upgrades_90d: 5,
            downgrades_90d: 2,
            net_90d: 3,
            latest_date: "2026-04-10".into(),
            latest_action: "upgrade".into(),
            bias_label: "BULLISH".into(),
            trend_label: "IMPROVING".into(),
            ..Default::default()
        };
        upsert_updm(&c, "NVDA", &snap).unwrap();
        let got = get_updm(&c, "nvda").unwrap().unwrap();
        assert_eq!(got.bias_label, "BULLISH");
        assert_eq!(got.net_90d, 3);
    }

    #[test]
    fn compute_updm_bullish_improving() {
        let actions = vec![
            rc("2026-04-10", "upgrade",   "Morgan Stanley", "Overweight"),
            rc("2026-04-05", "upgrade",   "Goldman Sachs",  "Buy"),
            rc("2026-03-28", "initiation","JPM",            "Overweight"),
            rc("2026-03-15", "downgrade", "Bernstein",      "Market-Perform"),
            rc("2026-02-20", "upgrade",   "Wells Fargo",    "Outperform"),
            rc("2025-12-10", "downgrade", "Citi",           "Neutral"),
        ];
        let snap = compute_updm_snapshot("NVDA", "2026-04-14", &actions);
        assert_eq!(snap.bias_label, "BULLISH");
        assert!(snap.net_90d > 0);
        assert_eq!(snap.latest_date, "2026-04-10");
        assert_eq!(snap.latest_action, "upgrade");
    }

    #[test]
    fn compute_updm_bearish() {
        let actions = vec![
            rc("2026-04-10", "downgrade", "Morgan Stanley", "Underweight"),
            rc("2026-04-01", "downgrade", "Goldman Sachs",  "Sell"),
            rc("2026-03-20", "downgrade", "Bernstein",      "Market-Perform"),
            rc("2026-03-10", "upgrade",   "Wells Fargo",    "Outperform"),
        ];
        let snap = compute_updm_snapshot("X", "2026-04-14", &actions);
        assert_eq!(snap.bias_label, "BEARISH");
        assert!(snap.net_90d < 0);
    }

    #[test]
    fn compute_updm_no_coverage() {
        let snap = compute_updm_snapshot("X", "2026-04-14", &[]);
        assert_eq!(snap.bias_label, "NO_COVERAGE");
    }

    // ── ADR-120 Godel Parity Round 13 tests ────────────────────────────────

    fn open_mem_conn_v13() -> Connection {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v13(&c).unwrap();
        c
    }

    fn make_bar(date: &str, close: f64, volume: f64) -> HistoricalPriceRow {
        HistoricalPriceRow {
            date: date.to_string(),
            open: close,
            high: close * 1.01,
            low: close * 0.99,
            close,
            adj_close: close,
            volume,
            change: 0.0,
            change_pct: 0.0,
        }
    }

    fn make_bar_flat(date: &str, close: f64, volume: f64) -> HistoricalPriceRow {
        HistoricalPriceRow {
            date: date.to_string(),
            open: close,
            high: close,
            low: close,
            close,
            adj_close: close,
            volume,
            change: 0.0,
            change_pct: 0.0,
        }
    }

    #[test]
    fn momentum_snapshot_roundtrip() {
        let c = open_mem_conn_v13();
        let snap = MomentumSnapshot {
            symbol: "X".to_string(),
            as_of: "2026-04-14".to_string(),
            bars_used: 252,
            return_12m_pct: 35.0,
            regime_label: "STRONG".to_string(),
            trend_label: "ACCELERATING".to_string(),
            ..Default::default()
        };
        upsert_momentum(&c, "x", &snap).unwrap();
        let got = get_momentum(&c, "X").unwrap().unwrap();
        assert_eq!(got.symbol, "X");
        assert_eq!(got.regime_label, "STRONG");
    }

    #[test]
    fn liquidity_snapshot_roundtrip() {
        let c = open_mem_conn_v13();
        let snap = LiquiditySnapshot {
            symbol: "Y".to_string(),
            as_of: "2026-04-14".to_string(),
            window_days: 60,
            avg_daily_dollar_volume: 1.5e9,
            liquidity_tier: "DEEP".to_string(),
            ..Default::default()
        };
        upsert_liquidity(&c, "y", &snap).unwrap();
        let got = get_liquidity(&c, "Y").unwrap().unwrap();
        assert_eq!(got.liquidity_tier, "DEEP");
    }

    #[test]
    fn breakout_snapshot_roundtrip() {
        let c = open_mem_conn_v13();
        let snap = BreakoutSnapshot {
            symbol: "Z".to_string(),
            as_of: "2026-04-14".to_string(),
            current_price: 100.0,
            high_52w: 100.0,
            low_52w: 60.0,
            position_in_52w_range_pct: 100.0,
            breakout_label: "NEW_HIGH".to_string(),
            setup_label: "TRENDING_UP".to_string(),
            ..Default::default()
        };
        upsert_breakout(&c, "z", &snap).unwrap();
        let got = get_breakout(&c, "Z").unwrap().unwrap();
        assert_eq!(got.breakout_label, "NEW_HIGH");
    }

    #[test]
    fn cash_cycle_snapshot_roundtrip() {
        let c = open_mem_conn_v13();
        let snap = CashCycleSnapshot {
            symbol: "W".to_string(),
            as_of: "2026-04-14".to_string(),
            latest_period: "2025-12-31".to_string(),
            ccc_days: 45.0,
            efficiency_label: "NEUTRAL".to_string(),
            trend_label: "STABLE".to_string(),
            periods: vec![CashCycleRow {
                period: "2025-12-31".to_string(),
                dso_days: 40.0,
                dio_days: 30.0,
                dpo_days: 25.0,
                ccc_days: 45.0,
            }],
            ..Default::default()
        };
        upsert_cash_cycle(&c, "w", &snap).unwrap();
        let got = get_cash_cycle(&c, "W").unwrap().unwrap();
        assert_eq!(got.latest_period, "2025-12-31");
        assert_eq!(got.periods.len(), 1);
    }

    #[test]
    fn credit_snapshot_roundtrip() {
        let c = open_mem_conn_v13();
        let snap = CreditSnapshot {
            symbol: "V".to_string(),
            as_of: "2026-04-14".to_string(),
            composite_score: 78.0,
            letter_grade: "A".to_string(),
            credit_label: "INVESTMENT_GRADE".to_string(),
            inputs_available: 4,
            ..Default::default()
        };
        upsert_credit(&c, "v", &snap).unwrap();
        let got = get_credit(&c, "V").unwrap().unwrap();
        assert_eq!(got.letter_grade, "A");
        assert_eq!(got.credit_label, "INVESTMENT_GRADE");
    }

    #[test]
    fn compute_momentum_strong() {
        // 260 bars, steadily rising from 100 → 140 with low vol.
        // Needs > 252 bars because compute_momentum_snapshot reads offset 252.
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        for i in 0..260 {
            // newest-first: i=0 is the newest bar
            let days_old = i as f64;
            let close = 140.0 - days_old * (40.0 / 260.0); // newest=140, oldest≈100
            bars.push(make_bar(&format!("2026-{:02}-{:02}", 1 + (i / 30) % 12, 1 + (i % 28)), close, 1_000_000.0));
        }
        let snap = compute_momentum_snapshot("AAA", "2026-04-14", &bars);
        assert!(snap.bars_used >= 252);
        assert!(snap.return_12m_pct > 0.0);
        assert_ne!(snap.regime_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_momentum_insufficient() {
        let bars: Vec<HistoricalPriceRow> = (0..50).map(|i| make_bar(&format!("2026-04-{:02}", (i % 28) + 1), 100.0, 1_000.0)).collect();
        let snap = compute_momentum_snapshot("AAA", "2026-04-14", &bars);
        assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_liquidity_deep() {
        // 60 bars, large dollar volume
        let bars: Vec<HistoricalPriceRow> = (0..60).map(|i| {
            make_bar(&format!("2026-04-{:02}", (i % 28) + 1), 200.0, 10_000_000.0)
        }).collect();
        let snap = compute_liquidity_snapshot("BBB", "2026-04-14", &bars, 1_000_000_000.0, 60);
        assert_eq!(snap.liquidity_tier, "DEEP");
        assert!(snap.avg_daily_dollar_volume > 1.0e9);
    }

    #[test]
    fn compute_liquidity_thin() {
        let bars: Vec<HistoricalPriceRow> = (0..60).map(|i| {
            make_bar(&format!("2026-04-{:02}", (i % 28) + 1), 5.0, 1_000.0)
        }).collect();
        let snap = compute_liquidity_snapshot("BBB", "2026-04-14", &bars, 100_000_000.0, 60);
        assert!(matches!(snap.liquidity_tier.as_str(), "THIN" | "ILLIQUID"));
    }

    #[test]
    fn compute_liquidity_insufficient() {
        let bars: Vec<HistoricalPriceRow> = (0..10).map(|i| make_bar(&format!("2026-04-{:02}", i + 1), 100.0, 1_000.0)).collect();
        let snap = compute_liquidity_snapshot("BBB", "2026-04-14", &bars, 1_000_000.0, 60);
        assert_eq!(snap.liquidity_tier, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_breakout_new_high() {
        // Uses flat bars (high = low = close) so `current >= h52` holds.
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        for i in 0..252 {
            let close = 100.0 - (i as f64) * 0.3; // newest = 100, oldest = ~24
            bars.push(make_bar_flat(&format!("2025-{:02}-{:02}", (1 + i / 30) % 12 + 1, (i % 28) + 1), close, 1_000_000.0));
        }
        let snap = compute_breakout_snapshot("CCC", "2026-04-14", &bars);
        assert_eq!(snap.breakout_label, "NEW_HIGH");
        assert!(snap.position_in_52w_range_pct >= 99.0);
    }

    #[test]
    fn compute_breakout_near_low() {
        // Newest bar is near the 52w low (older bars are higher)
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        for i in 0..252 {
            let close = 10.0 + (i as f64) * 0.3;
            bars.push(make_bar_flat(&format!("2025-{:02}-{:02}", (1 + i / 30) % 12 + 1, (i % 28) + 1), close, 1_000_000.0));
        }
        let snap = compute_breakout_snapshot("CCC", "2026-04-14", &bars);
        assert!(matches!(snap.breakout_label.as_str(), "NEAR_LOW" | "NEW_LOW"));
    }

    #[test]
    fn compute_breakout_insufficient() {
        let bars: Vec<HistoricalPriceRow> = (0..5).map(|i| make_bar(&format!("2026-04-{:02}", i + 1), 100.0, 1_000.0)).collect();
        let snap = compute_breakout_snapshot("CCC", "2026-04-14", &bars);
        assert_eq!(snap.breakout_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_cash_cycle_efficient() {
        let income = IncomeStatement {
            date: "2025-12-31".to_string(),
            period: "FY".to_string(),
            revenue: 10_000.0,
            cost_of_revenue: 6_000.0,
            ..Default::default()
        };
        let income_prior = IncomeStatement {
            date: "2024-12-31".to_string(),
            period: "FY".to_string(),
            revenue: 9_000.0,
            cost_of_revenue: 5_400.0,
            ..Default::default()
        };
        let balance = BalanceSheet {
            date: "2025-12-31".to_string(),
            period: "FY".to_string(),
            net_receivables: 400.0,   // ~14.6 DSO
            inventory: 300.0,         // ~18.25 DIO
            accounts_payable: 900.0,  // ~54.75 DPO → CCC ≈ -21.9
            ..Default::default()
        };
        let balance_prior = BalanceSheet {
            date: "2024-12-31".to_string(),
            period: "FY".to_string(),
            net_receivables: 500.0,
            inventory: 350.0,
            accounts_payable: 850.0,
            ..Default::default()
        };
        let statements = FinancialStatements {
            income_annual: vec![income, income_prior],
            balance_annual: vec![balance, balance_prior],
            ..Default::default()
        };
        let snap = compute_cash_cycle_snapshot("DDD", "2026-04-14", &statements);
        assert!(snap.ccc_days < 30.0);
        assert_eq!(snap.efficiency_label, "EFFICIENT");
        assert_eq!(snap.periods.len(), 2);
    }

    #[test]
    fn compute_cash_cycle_insufficient() {
        let statements = FinancialStatements::default();
        let snap = compute_cash_cycle_snapshot("DDD", "2026-04-14", &statements);
        assert_eq!(snap.efficiency_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_credit_investment_grade() {
        let altz = AltmanZSnapshot {
            z_score: 4.5,
            zone: "SAFE".to_string(),
            ..Default::default()
        };
        let ptfs = PiotroskiSnapshot {
            f_score: 8,
            strength_label: "STRONG".to_string(),
            ..Default::default()
        };
        let lev = LeverageSnapshot {
            solvency_summary: "HEALTHY".to_string(),
            ..Default::default()
        };
        let acrl = AccrualsSnapshot {
            trend_label: "IMPROVING".to_string(),
            ttm_cash_conversion_pct: 120.0,
            ..Default::default()
        };
        let snap = compute_credit_snapshot("EEE", "2026-04-14", Some(&altz), Some(&ptfs), Some(&lev), Some(&acrl));
        assert_eq!(snap.credit_label, "INVESTMENT_GRADE");
        assert!(snap.composite_score >= 70.0);
        assert_eq!(snap.inputs_available, 4);
        assert_eq!(snap.components.len(), 4);
    }

    #[test]
    fn compute_credit_distressed() {
        let altz = AltmanZSnapshot {
            z_score: 0.8,
            zone: "DISTRESS".to_string(),
            ..Default::default()
        };
        let ptfs = PiotroskiSnapshot {
            f_score: 1,
            strength_label: "WEAK".to_string(),
            ..Default::default()
        };
        let lev = LeverageSnapshot {
            solvency_summary: "STRETCHED".to_string(),
            ..Default::default()
        };
        let acrl = AccrualsSnapshot {
            trend_label: "DETERIORATING".to_string(),
            ttm_cash_conversion_pct: 30.0,
            ..Default::default()
        };
        let snap = compute_credit_snapshot("EEE", "2026-04-14", Some(&altz), Some(&ptfs), Some(&lev), Some(&acrl));
        assert_eq!(snap.credit_label, "DISTRESSED");
        assert!(snap.composite_score < 35.0);
    }

    #[test]
    fn compute_credit_no_inputs() {
        let snap = compute_credit_snapshot("EEE", "2026-04-14", None, None, None, None);
        assert_eq!(snap.letter_grade, "INSUFFICIENT_DATA");
        assert_eq!(snap.inputs_available, 0);
    }

    // ── ADR-121 Round 14 tests ─────────────────────────────────────────────

    #[test]
    fn growm_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v14(&c).unwrap();
        let snap = GrowmSnapshot {
            symbol: "AAA".to_string(),
            as_of: "2026-04-14".to_string(),
            composite_score: 82.5,
            garp_label: "GARP".to_string(),
            inputs_available: 3,
            ..Default::default()
        };
        upsert_growm(&c, "aaa", &snap).unwrap();
        let got = get_growm(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.garp_label, "GARP");
        assert_eq!(got.inputs_available, 3);
    }

    #[test]
    fn flow_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v14(&c).unwrap();
        let snap = FlowSnapshot {
            symbol: "BBB".to_string(),
            as_of: "2026-04-14".to_string(),
            window_days: 90,
            composite_score: 72.0,
            flow_label: "BUY".to_string(),
            ..Default::default()
        };
        upsert_flow(&c, "bbb", &snap).unwrap();
        let got = get_flow(&c, "BBB").unwrap().unwrap();
        assert_eq!(got.flow_label, "BUY");
    }

    #[test]
    fn regime_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v14(&c).unwrap();
        let snap = RegimeSnapshot {
            symbol: "CCC".to_string(),
            as_of: "2026-04-14".to_string(),
            regime_label: "TRENDING".to_string(),
            inputs_available: 3,
            ..Default::default()
        };
        upsert_regime(&c, "ccc", &snap).unwrap();
        let got = get_regime(&c, "CCC").unwrap().unwrap();
        assert_eq!(got.regime_label, "TRENDING");
    }

    #[test]
    fn relvol_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v14(&c).unwrap();
        let snap = RelVolSnapshot {
            symbol: "DDD".to_string(),
            as_of: "2026-04-14".to_string(),
            activity_label: "HIGH".to_string(),
            direction_label: "BULLISH".to_string(),
            bars_used: 60,
            ..Default::default()
        };
        upsert_relvol(&c, "ddd", &snap).unwrap();
        let got = get_relvol(&c, "DDD").unwrap().unwrap();
        assert_eq!(got.activity_label, "HIGH");
    }

    #[test]
    fn margins_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v14(&c).unwrap();
        let snap = MarginsSnapshot {
            symbol: "EEE".to_string(),
            as_of: "2026-04-14".to_string(),
            basis: "annual".to_string(),
            overall_trend_label: "EXPANDING".to_string(),
            quality_label: "HIGH".to_string(),
            ..Default::default()
        };
        upsert_margins(&c, "eee", &snap).unwrap();
        let got = get_margins(&c, "EEE").unwrap().unwrap();
        assert_eq!(got.overall_trend_label, "EXPANDING");
    }

    #[test]
    fn compute_growm_garp() {
        let mom = MomentumSnapshot {
            composite_score: 78.0,
            regime_label: "STRONG".to_string(),
            ..Default::default()
        };
        let earm = EarmSnapshot {
            composite_score: 72.0,
            momentum_label: "ACCELERATING".to_string(),
            ..Default::default()
        };
        let divg = DivgSnapshot {
            cagr_3y_pct: 8.0,
            trend_label: "GROWING".to_string(),
            ..Default::default()
        };
        let snap = compute_growm_snapshot("AAA", "2026-04-14", Some(&mom), Some(&earm), Some(&divg));
        assert!(snap.composite_score >= 65.0);
        assert_eq!(snap.inputs_available, 3);
        assert!(matches!(snap.garp_label.as_str(), "GARP" | "GROWTH"));
    }

    #[test]
    fn compute_growm_no_inputs() {
        let snap = compute_growm_snapshot("AAA", "2026-04-14", None, None, None);
        assert_eq!(snap.garp_label, "NO_DATA");
        assert_eq!(snap.inputs_available, 0);
    }

    #[test]
    fn compute_flow_buy() {
        let trades = vec![
            InsiderTrade {
                transaction_date: "2026-04-10".to_string(),
                reporting_name: "Alice CFO".to_string(),
                transaction_type: "P-Purchase".to_string(),
                value_usd: 500_000.0,
                ..Default::default()
            },
            InsiderTrade {
                transaction_date: "2026-04-01".to_string(),
                reporting_name: "Bob CEO".to_string(),
                transaction_type: "P-Purchase".to_string(),
                value_usd: 800_000.0,
                ..Default::default()
            },
        ];
        let holders = vec![
            InstitutionalHolder { holder: "X Fund".to_string(), change: 100_000.0, ..Default::default() },
            InstitutionalHolder { holder: "Y Fund".to_string(), change: 50_000.0, ..Default::default() },
            InstitutionalHolder { holder: "Z Fund".to_string(), change: -20_000.0, ..Default::default() },
        ];
        let snap = compute_flow_snapshot("BBB", "2026-04-14", &trades, &holders, 90);
        assert!(matches!(snap.flow_label.as_str(), "BUY" | "STRONG_BUY"));
        assert_eq!(snap.insider_trade_count, 2);
        assert_eq!(snap.institutional_buyers, 2);
        assert_eq!(snap.institutional_sellers, 1);
    }

    #[test]
    fn compute_flow_no_data() {
        let snap = compute_flow_snapshot("BBB", "2026-04-14", &[], &[], 90);
        assert_eq!(snap.flow_label, "NO_DATA");
    }

    #[test]
    fn compute_regime_trending() {
        let tech = TechnicalSnapshot {
            indicators: vec![
                TechnicalIndicator { name: "ADX(14)".to_string(), value: 32.0, ..Default::default() },
            ],
            trend_summary: "bullish trend".to_string(),
            ..Default::default()
        };
        let vole = OhlcVolSnapshot {
            preferred_estimate_pct: 18.0,
            preferred_label: "Yang-Zhang".to_string(),
            ..Default::default()
        };
        let hra = HraSnapshot {
            sharpe_ratio: 1.8,
            volatility_annual_pct: 18.0,
            windows: vec![HraWindow { label: "1Y".to_string(), return_pct: 22.0, ..Default::default() }],
            ..Default::default()
        };
        let snap = compute_regime_snapshot("CCC", "2026-04-14", Some(&vole), Some(&tech), Some(&hra));
        assert_eq!(snap.regime_label, "TRENDING");
        assert_eq!(snap.inputs_available, 3);
    }

    #[test]
    fn compute_regime_volatile() {
        let vole = OhlcVolSnapshot {
            preferred_estimate_pct: 55.0,
            preferred_label: "Yang-Zhang".to_string(),
            ..Default::default()
        };
        let snap = compute_regime_snapshot("CCC", "2026-04-14", Some(&vole), None, None);
        assert_eq!(snap.regime_label, "VOLATILE");
    }

    #[test]
    fn compute_regime_no_inputs() {
        let snap = compute_regime_snapshot("CCC", "2026-04-14", None, None, None);
        assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_relvol_high() {
        let mut bars: Vec<HistoricalPriceRow> = Vec::new();
        // Current bar (index 0) has 5x avg volume.
        bars.push(HistoricalPriceRow {
            date: "2026-04-14".to_string(),
            volume: 5_000_000.0,
            close: 105.0,
            ..Default::default()
        });
        for i in 1..=60 {
            bars.push(HistoricalPriceRow {
                date: format!("2026-04-{:02}", 14 - (i % 14)),
                volume: 1_000_000.0,
                close: 100.0,
                ..Default::default()
            });
        }
        let snap = compute_relvol_snapshot("DDD", "2026-04-14", &bars);
        assert!(snap.rel_volume_20d >= 4.0);
        assert_eq!(snap.activity_label, "EXTREME");
        assert_eq!(snap.direction_label, "BULLISH");
    }

    #[test]
    fn compute_relvol_insufficient() {
        let bars: Vec<HistoricalPriceRow> = (0..10).map(|i| HistoricalPriceRow {
            date: format!("2026-04-{:02}", 14 - i),
            volume: 1_000.0,
            close: 100.0,
            ..Default::default()
        }).collect();
        let snap = compute_relvol_snapshot("DDD", "2026-04-14", &bars);
        assert_eq!(snap.activity_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_margins_expanding() {
        let latest = IncomeStatement {
            date: "2025-12-31".to_string(),
            period: "FY".to_string(),
            revenue: 10_000.0,
            gross_profit: 4_500.0,     // 45%
            operating_income: 2_500.0, // 25%
            net_income: 1_800.0,       // 18%
            ..Default::default()
        };
        let prior = IncomeStatement {
            date: "2024-12-31".to_string(),
            period: "FY".to_string(),
            revenue: 9_000.0,
            gross_profit: 3_600.0,     // 40%
            operating_income: 1_800.0, // 20%
            net_income: 1_350.0,       // 15%
            ..Default::default()
        };
        let statements = FinancialStatements {
            income_annual: vec![latest, prior],
            ..Default::default()
        };
        let snap = compute_margins_snapshot("EEE", "2026-04-14", &statements);
        assert_eq!(snap.overall_trend_label, "EXPANDING");
        assert_eq!(snap.quality_label, "HIGH");
        assert_eq!(snap.periods_used, 2);
        assert!(snap.operating_margin_change_pct > 0.0);
    }

    #[test]
    fn compute_margins_insufficient() {
        let statements = FinancialStatements::default();
        let snap = compute_margins_snapshot("EEE", "2026-04-14", &statements);
        assert_eq!(snap.overall_trend_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-122 Round 15 tests ─────────────────────────────────────────────

    #[test]
    fn val_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v15(&c).unwrap();
        let snap = ValueSnapshot {
            symbol: "VAL1".to_string(),
            as_of: "2026-04-14".to_string(),
            sector: "Technology".to_string(),
            peers_considered: 9,
            value_label: "VALUE".to_string(),
            ..Default::default()
        };
        upsert_val(&c, "val1", &snap).unwrap();
        let got = get_val(&c, "VAL1").unwrap().unwrap();
        assert_eq!(got.value_label, "VALUE");
        assert_eq!(got.peers_considered, 9);
    }

    #[test]
    fn qual_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v15(&c).unwrap();
        let snap = QualitySnapshot {
            symbol: "QL1".to_string(),
            as_of: "2026-04-14".to_string(),
            quality_label: "HIGH_QUALITY".to_string(),
            composite_score: 82.0,
            ..Default::default()
        };
        upsert_qual(&c, "ql1", &snap).unwrap();
        let got = get_qual(&c, "QL1").unwrap().unwrap();
        assert_eq!(got.quality_label, "HIGH_QUALITY");
    }

    #[test]
    fn risk_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v15(&c).unwrap();
        let snap = RiskSnapshot {
            symbol: "RK1".to_string(),
            as_of: "2026-04-14".to_string(),
            risk_label: "MODERATE".to_string(),
            composite_score: 42.0,
            ..Default::default()
        };
        upsert_risk(&c, "rk1", &snap).unwrap();
        let got = get_risk(&c, "RK1").unwrap().unwrap();
        assert_eq!(got.risk_label, "MODERATE");
    }

    #[test]
    fn insstrk_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v15(&c).unwrap();
        let snap = InsiderStreakSnapshot {
            symbol: "INS1".to_string(),
            as_of: "2026-04-14".to_string(),
            window_days: 180,
            unique_insiders: 4,
            streak_label: "ACCUMULATION".to_string(),
            ..Default::default()
        };
        upsert_insstrk(&c, "ins1", &snap).unwrap();
        let got = get_insstrk(&c, "INS1").unwrap().unwrap();
        assert_eq!(got.streak_label, "ACCUMULATION");
    }

    #[test]
    fn covg_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v15(&c).unwrap();
        let snap = CoverageSnapshot {
            symbol: "CVG1".to_string(),
            as_of: "2026-04-14".to_string(),
            num_analysts: 18,
            coverage_label: "STABLE".to_string(),
            ..Default::default()
        };
        upsert_covg(&c, "cvg1", &snap).unwrap();
        let got = get_covg(&c, "CVG1").unwrap().unwrap();
        assert_eq!(got.coverage_label, "STABLE");
    }

    #[test]
    fn compute_val_value_label() {
        use crate::core::fundamentals::Fundamentals;
        let subject = Fundamentals {
            symbol: "SUB".to_string(),
            pe_ratio: Some(10.0),
            forward_pe: Some(9.0),
            price_to_book: Some(1.0),
            price_to_sales: Some(1.0),
            ev_to_ebitda: Some(6.0),
            ..Default::default()
        };
        let peers: Vec<Fundamentals> = (0..5).map(|i| Fundamentals {
            symbol: format!("P{}", i),
            pe_ratio: Some(20.0),
            forward_pe: Some(18.0),
            price_to_book: Some(3.0),
            price_to_sales: Some(3.0),
            ev_to_ebitda: Some(14.0),
            ..Default::default()
        }).collect();
        let fcfy = FcfYieldSnapshot {
            ttm_fcf_yield_pct: 8.0,
            ..Default::default()
        };
        let peer_fcfy = vec![4.0, 4.0, 4.0, 4.0, 4.0];
        let snap = compute_val_snapshot("SUB", "2026-04-14", "Technology", Some(&subject), &peers, Some(&fcfy), &peer_fcfy);
        assert!(matches!(snap.value_label.as_str(), "DEEP_VALUE" | "VALUE"));
        assert!(snap.composite_score >= 65.0);
        assert_eq!(snap.inputs_available, 6);
    }

    #[test]
    fn compute_val_no_data() {
        let snap = compute_val_snapshot("SUB", "2026-04-14", "Technology", None, &[], None, &[]);
        assert_eq!(snap.value_label, "NO_DATA");
    }

    #[test]
    fn compute_qual_high_quality() {
        let pt = PiotroskiSnapshot {
            f_score: 8,
            strength_label: "STRONG".to_string(),
            ..Default::default()
        };
        let mg = MarginsSnapshot {
            latest_operating_margin_pct: 28.0,
            overall_trend_label: "EXPANDING".to_string(),
            quality_label: "HIGH".to_string(),
            ..Default::default()
        };
        let ac = AccrualsSnapshot {
            ttm_cash_conversion_pct: 115.0,
            trend_label: "IMPROVING".to_string(),
            ..Default::default()
        };
        let lv = LeverageSnapshot {
            total_debt: 100.0,
            ebitda_ttm: 200.0,
            solvency_summary: "HEALTHY".to_string(),
            ..Default::default()
        };
        let snap = compute_qual_snapshot("QQ", "2026-04-14", Some(&pt), Some(&mg), Some(&ac), Some(&lv));
        assert!(matches!(snap.quality_label.as_str(), "HIGH_QUALITY" | "QUALITY"));
        assert_eq!(snap.inputs_available, 4);
        assert!(snap.composite_score >= 70.0);
    }

    #[test]
    fn compute_qual_no_inputs() {
        let snap = compute_qual_snapshot("QQ", "2026-04-14", None, None, None, None);
        assert_eq!(snap.quality_label, "NO_DATA");
    }

    #[test]
    fn compute_risk_distressed() {
        let altz = AltmanZSnapshot {
            z_score: 1.2,
            zone: "DISTRESS".to_string(),
            ..Default::default()
        };
        let snap = compute_risk_snapshot("RK", "2026-04-14", None, None, None, None, Some(&altz));
        assert_eq!(snap.risk_label, "DISTRESSED");
    }

    #[test]
    fn compute_risk_low() {
        let vole = OhlcVolSnapshot {
            preferred_estimate_pct: 12.0,
            preferred_label: "Yang-Zhang".to_string(),
            ..Default::default()
        };
        let beta = BetaSnapshot {
            windows: vec![BetaWindow {
                window_label: "1Y".to_string(),
                beta: 1.05,
                n_observations: 252,
                ..Default::default()
            }],
            ..Default::default()
        };
        let liq = LiquiditySnapshot {
            liquidity_tier: "DEEP".to_string(),
            ..Default::default()
        };
        let altz = AltmanZSnapshot {
            z_score: 4.5,
            zone: "SAFE".to_string(),
            ..Default::default()
        };
        let snap = compute_risk_snapshot("RK", "2026-04-14", Some(&vole), Some(&beta), Some(&liq), None, Some(&altz));
        assert_eq!(snap.risk_label, "LOW_RISK");
        assert_eq!(snap.inputs_available, 4);
    }

    #[test]
    fn compute_risk_no_inputs() {
        let snap = compute_risk_snapshot("RK", "2026-04-14", None, None, None, None, None);
        assert_eq!(snap.risk_label, "NO_DATA");
    }

    #[test]
    fn compute_insstrk_accumulation() {
        let trades = vec![
            InsiderTrade {
                transaction_date: "2026-03-01".to_string(),
                reporting_name: "Alice CFO".to_string(),
                transaction_type: "P-Purchase".to_string(),
                acquisition_disposition: "A".to_string(),
                shares: 1_000.0,
                value_usd: 50_000.0,
                ..Default::default()
            },
            InsiderTrade {
                transaction_date: "2026-03-10".to_string(),
                reporting_name: "Alice CFO".to_string(),
                transaction_type: "P-Purchase".to_string(),
                acquisition_disposition: "A".to_string(),
                shares: 1_000.0,
                value_usd: 55_000.0,
                ..Default::default()
            },
            InsiderTrade {
                transaction_date: "2026-03-05".to_string(),
                reporting_name: "Bob CEO".to_string(),
                transaction_type: "P-Purchase".to_string(),
                acquisition_disposition: "A".to_string(),
                shares: 500.0,
                value_usd: 25_000.0,
                ..Default::default()
            },
            InsiderTrade {
                transaction_date: "2026-03-20".to_string(),
                reporting_name: "Bob CEO".to_string(),
                transaction_type: "P-Purchase".to_string(),
                acquisition_disposition: "A".to_string(),
                shares: 500.0,
                value_usd: 27_000.0,
                ..Default::default()
            },
        ];
        let snap = compute_insstrk_snapshot("INS", "2026-04-14", &trades, 180);
        assert_eq!(snap.unique_insiders, 2);
        assert!(matches!(snap.streak_label.as_str(), "ACCUMULATION" | "STRONG_ACCUMULATION" | "MIXED"));
        assert!(snap.buy_streak_count >= 2);
    }

    #[test]
    fn compute_insstrk_none() {
        let snap = compute_insstrk_snapshot("INS", "2026-04-14", &[], 180);
        assert_eq!(snap.streak_label, "NONE");
    }

    #[test]
    fn compute_covg_stable() {
        let pt = PriceTarget {
            symbol: "CVG".to_string(),
            target_mean: 150.0,
            target_low: 120.0,
            target_high: 180.0,
            num_analysts: 18,
            ..Default::default()
        };
        let recs = vec![
            AnalystRecommendation {
                period: "2026-04-01".to_string(),
                strong_buy: 6,
                buy: 8,
                hold: 3,
                sell: 1,
                strong_sell: 0,
            },
        ];
        let updm = UpdmSnapshot {
            total_actions: 10,
            upgrades_90d: 5,
            downgrades_90d: 3,
            net_90d: 2,
            ..Default::default()
        };
        let snap = compute_covg_snapshot("CVG", "2026-04-14", Some(&pt), &recs, Some(&updm));
        assert!(matches!(snap.coverage_label.as_str(), "STABLE" | "EXPANDING"));
        assert_eq!(snap.inputs_available, 3);
    }

    #[test]
    fn compute_covg_none() {
        let snap = compute_covg_snapshot("CVG", "2026-04-14", None, &[], None);
        assert_eq!(snap.coverage_label, "NONE");
    }

    // ── ADR-123 Round 16 tests ─────────────────────────────────────────────

    #[test]
    fn vrk_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v16(&c).unwrap();
        let snap = ValueRankSnapshot {
            symbol: "VRK1".into(),
            as_of: "2026-04-15".into(),
            sector: "Tech".into(),
            rank_label: "TOP_QUARTILE".into(),
            percentile_rank: 78.5,
            ..Default::default()
        };
        upsert_vrk(&c, "vrk1", &snap).unwrap();
        let got = get_vrk(&c, "VRK1").unwrap().unwrap();
        assert_eq!(got.rank_label, "TOP_QUARTILE");
        assert!((got.percentile_rank - 78.5).abs() < 1e-6);
    }

    #[test]
    fn qrk_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v16(&c).unwrap();
        let snap = QualityRankSnapshot {
            symbol: "QRK1".into(),
            as_of: "2026-04-15".into(),
            sector: "Healthcare".into(),
            rank_label: "ABOVE_MEDIAN".into(),
            ..Default::default()
        };
        upsert_qrk(&c, "qrk1", &snap).unwrap();
        let got = get_qrk(&c, "QRK1").unwrap().unwrap();
        assert_eq!(got.rank_label, "ABOVE_MEDIAN");
    }

    #[test]
    fn rrk_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v16(&c).unwrap();
        let snap = RiskRankSnapshot {
            symbol: "RRK1".into(),
            as_of: "2026-04-15".into(),
            sector: "Energy".into(),
            rank_label: "SAFEST_QUARTILE".into(),
            ..Default::default()
        };
        upsert_rrk(&c, "rrk1", &snap).unwrap();
        let got = get_rrk(&c, "RRK1").unwrap().unwrap();
        assert_eq!(got.rank_label, "SAFEST_QUARTILE");
    }

    #[test]
    fn relepsgr_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v16(&c).unwrap();
        let snap = RelativeEpsGrowthSnapshot {
            symbol: "RELEPS1".into(),
            as_of: "2026-04-15".into(),
            sector: "Tech".into(),
            symbol_cagr_pct: 22.0,
            sector_median_cagr_pct: 12.0,
            relative_label: "ABOVE".into(),
            ..Default::default()
        };
        upsert_relepsgr(&c, "releps1", &snap).unwrap();
        let got = get_relepsgr(&c, "RELEPS1").unwrap().unwrap();
        assert_eq!(got.relative_label, "ABOVE");
    }

    #[test]
    fn pead_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v16(&c).unwrap();
        let snap = PeadSnapshot {
            symbol: "PEAD1".into(),
            as_of: "2026-04-15".into(),
            num_events: 8,
            events_used: 6,
            avg_drift_5d_pct: 3.2,
            drift_direction_label: "DRIFT_UP".into(),
            ..Default::default()
        };
        upsert_pead(&c, "pead1", &snap).unwrap();
        let got = get_pead(&c, "PEAD1").unwrap().unwrap();
        assert_eq!(got.drift_direction_label, "DRIFT_UP");
    }

    fn mk_val_snap(sym: &str, sector: &str, score: f64) -> ValueSnapshot {
        ValueSnapshot {
            symbol: sym.to_string(),
            as_of: "2026-04-15".into(),
            sector: sector.to_string(),
            composite_score: score,
            value_label: if score >= 65.0 { "VALUE".into() } else { "FAIR".into() },
            inputs_available: 6,
            ..Default::default()
        }
    }

    #[test]
    fn compute_vrk_top_decile() {
        let subj = mk_val_snap("SUB", "Tech", 92.0);
        let peer_vec: Vec<ValueSnapshot> = (0..9)
            .map(|i| mk_val_snap(&format!("P{}", i), "Tech", 30.0 + i as f64 * 5.0))
            .collect();
        let peers: Vec<&ValueSnapshot> = peer_vec.iter().collect();
        let snap = compute_vrk_snapshot("SUB", "2026-04-15", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert!(snap.percentile_rank >= 90.0);
        assert_eq!(snap.rank_position, 1);
        assert_eq!(snap.peers_considered, 9);
    }

    #[test]
    fn compute_vrk_insufficient_peers() {
        let subj = mk_val_snap("SUB", "Tech", 70.0);
        let peer_vec = vec![mk_val_snap("P1", "Tech", 50.0), mk_val_snap("P2", "Tech", 60.0)];
        let peers: Vec<&ValueSnapshot> = peer_vec.iter().collect();
        let snap = compute_vrk_snapshot("SUB", "2026-04-15", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    fn mk_qual_snap(sym: &str, score: f64) -> QualitySnapshot {
        QualitySnapshot {
            symbol: sym.to_string(),
            as_of: "2026-04-15".into(),
            composite_score: score,
            quality_label: if score >= 65.0 { "QUALITY".into() } else { "AVERAGE".into() },
            inputs_available: 4,
            ..Default::default()
        }
    }

    #[test]
    fn compute_qrk_above_median() {
        let subj = mk_qual_snap("SUB", 72.0);
        let peer_vec: Vec<QualitySnapshot> = (0..9)
            .map(|i| mk_qual_snap(&format!("P{}", i), 40.0 + i as f64 * 4.0))
            .collect();
        let peers: Vec<&QualitySnapshot> = peer_vec.iter().collect();
        let snap = compute_qrk_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &peers);
        assert!(matches!(
            snap.rank_label.as_str(),
            "ABOVE_MEDIAN" | "TOP_QUARTILE" | "TOP_DECILE"
        ));
        assert!(snap.percentile_rank >= 50.0);
    }

    #[test]
    fn compute_qrk_no_data() {
        let snap = compute_qrk_snapshot("SUB", "2026-04-15", "Tech", None, &[]);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    fn mk_risk_snap(sym: &str, composite: f64) -> RiskSnapshot {
        RiskSnapshot {
            symbol: sym.to_string(),
            as_of: "2026-04-15".into(),
            composite_score: composite,
            risk_label: if composite >= 75.0 { "HIGH_RISK".into() } else { "MODERATE".into() },
            inputs_available: 5,
            ..Default::default()
        }
    }

    #[test]
    fn compute_rrk_safest() {
        // Subject has the LOWEST risk composite in the cohort → SAFEST_DECILE.
        let subj = mk_risk_snap("SUB", 10.0);
        let peer_vec: Vec<RiskSnapshot> = (0..9)
            .map(|i| mk_risk_snap(&format!("P{}", i), 40.0 + i as f64 * 5.0))
            .collect();
        let peers: Vec<&RiskSnapshot> = peer_vec.iter().collect();
        let snap = compute_rrk_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "SAFEST_DECILE");
        assert!(snap.percentile_rank >= 90.0);
        assert_eq!(snap.rank_position, 1);
    }

    #[test]
    fn compute_rrk_riskiest() {
        // Subject has the HIGHEST risk composite → RISKIEST_DECILE.
        let subj = mk_risk_snap("SUB", 95.0);
        let peer_vec: Vec<RiskSnapshot> = (0..9)
            .map(|i| mk_risk_snap(&format!("P{}", i), 20.0 + i as f64 * 5.0))
            .collect();
        let peers: Vec<&RiskSnapshot> = peer_vec.iter().collect();
        let snap = compute_rrk_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "RISKIEST_DECILE");
        assert!(snap.percentile_rank < 10.0);
    }

    fn mk_financials_with_eps(annual_eps_newest_first: &[f64]) -> FinancialStatements {
        let income: Vec<IncomeStatement> = annual_eps_newest_first
            .iter()
            .enumerate()
            .map(|(i, &eps)| IncomeStatement {
                date: format!("202{}-12-31", 6 - i),
                period: "FY".into(),
                eps,
                eps_diluted: eps,
                ..Default::default()
            })
            .collect();
        FinancialStatements {
            income_annual: income,
            ..Default::default()
        }
    }

    #[test]
    fn compute_relepsgr_above() {
        // Subject EPS: 8 → 4 (newest to oldest), 3-year CAGR ≈ 26 %.
        let subj = mk_financials_with_eps(&[8.0, 7.0, 5.5, 4.0]);
        // Peers: flat ~12 % CAGR (2.0 → 1.42)
        let peer_vec: Vec<(String, FinancialStatements)> = (0..5)
            .map(|i| (format!("P{}", i), mk_financials_with_eps(&[2.0, 1.8, 1.6, 1.42])))
            .collect();
        let snap = compute_relepsgr_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &peer_vec);
        assert!(matches!(snap.relative_label.as_str(), "ABOVE" | "FAR_ABOVE"));
        assert!(snap.symbol_cagr_pct > snap.sector_median_cagr_pct);
        assert_eq!(snap.years_used, 3);
    }

    #[test]
    fn compute_relepsgr_insufficient() {
        let subj = mk_financials_with_eps(&[5.0, 4.0]); // only 2 rows
        let snap = compute_relepsgr_snapshot("SUB", "2026-04-15", "Tech", Some(&subj), &[]);
        assert_eq!(snap.relative_label, "NO_DATA");
    }

    #[test]
    fn compute_pead_drift_up() {
        // Build 15 newest-first HP bars with a steady 1%/day advance.
        let mut bars: Vec<HistoricalPriceRow> = (0..15)
            .map(|i| HistoricalPriceRow {
                date: format!("2026-04-{:02}", 15 - i),
                close: 100.0 * (1.01f64).powi((14 - i) as i32),
                ..Default::default()
            })
            .collect();
        // newest-first
        bars.sort_by(|a, b| b.date.cmp(&a.date));
        // Build 3 beat surprises at increasing dates (all old enough that t0_idx ≥ 10).
        let surprises = vec![
            EarningsSurprise {
                date: "2026-04-01".into(),
                symbol: "PEAD".into(),
                eps_actual: 1.10,
                eps_estimate: 1.00,
                surprise: 0.10,
                surprise_pct: 10.0,
            },
            EarningsSurprise {
                date: "2026-04-02".into(),
                symbol: "PEAD".into(),
                eps_actual: 1.20,
                eps_estimate: 1.00,
                surprise: 0.20,
                surprise_pct: 20.0,
            },
            EarningsSurprise {
                date: "2026-04-03".into(),
                symbol: "PEAD".into(),
                eps_actual: 1.15,
                eps_estimate: 1.00,
                surprise: 0.15,
                surprise_pct: 15.0,
            },
        ];
        let snap = compute_pead_snapshot("PEAD", "2026-04-15", &surprises, &bars);
        assert_eq!(snap.drift_direction_label, "DRIFT_UP");
        assert!(snap.events_used >= 3);
        assert!(snap.avg_drift_5d_pct > 2.0);
    }

    #[test]
    fn compute_pead_no_events() {
        let snap = compute_pead_snapshot("PEAD", "2026-04-15", &[], &[]);
        assert_eq!(snap.drift_direction_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-124 Round 17 tests ────────────────────────────────────────────

    fn mk_mom(sym: &str, composite: f64) -> MomentumSnapshot {
        MomentumSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            bars_used: 252,
            composite_score: composite,
            regime_label: "STRONG".into(),
            trend_label: "STABLE".into(),
            ..Default::default()
        }
    }

    fn mk_pead(sym: &str, avg_5d: f64, events: usize) -> PeadSnapshot {
        PeadSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            num_events: events,
            events_used: events,
            avg_drift_5d_pct: avg_5d,
            drift_direction_label: if avg_5d > 0.5 { "DRIFT_UP".into() } else { "MIXED".into() },
            ..Default::default()
        }
    }

    fn mk_ptfs(sym: &str, f_score: i32) -> PiotroskiSnapshot {
        PiotroskiSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            f_score,
            strength_label: if f_score >= 7 { "STRONG".into() } else if f_score >= 4 { "MIXED".into() } else { "WEAK".into() },
            ..Default::default()
        }
    }

    fn mk_margins_q(sym: &str, op_margin: f64, quality: &str, trend: &str) -> MarginsSnapshot {
        MarginsSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            latest_operating_margin_pct: op_margin,
            quality_label: quality.into(),
            overall_trend_label: trend.into(),
            ..Default::default()
        }
    }

    fn mk_accruals(sym: &str, cash_conv: f64, trend: &str) -> AccrualsSnapshot {
        AccrualsSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            ttm_cash_conversion_pct: cash_conv,
            trend_label: trend.into(),
            ..Default::default()
        }
    }

    fn mk_financials_with_revenue(_sym: &str, revs: &[f64]) -> FinancialStatements {
        let income_annual: Vec<IncomeStatement> = revs
            .iter()
            .enumerate()
            .map(|(i, r)| IncomeStatement {
                date: format!("202{}-12-31", 6 - i),
                period: "FY".into(),
                revenue: *r,
                ..Default::default()
            })
            .collect();
        FinancialStatements {
            income_annual,
            ..Default::default()
        }
    }

    #[test]
    fn sizef_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v17(&conn).expect("create v17");
        let snap = SizeFactorSnapshot {
            symbol: "SIZ".into(),
            as_of: "2026-04-15".into(),
            sector: "Technology".into(),
            market_cap: 5e11,
            tier_label: "MEGA_CAP".into(),
            percentile_rank: 92.0,
            rank_label: "TOP_DECILE".into(),
            ..Default::default()
        };
        upsert_sizef(&conn, "SIZ", &snap).unwrap();
        let got = get_sizef(&conn, "SIZ").unwrap().unwrap();
        assert_eq!(got.tier_label, "MEGA_CAP");
        assert_eq!(got.rank_label, "TOP_DECILE");
    }

    #[test]
    fn momf_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v17(&conn).expect("create v17");
        let snap = MomentumRankSnapshot {
            symbol: "MMF".into(),
            as_of: "2026-04-15".into(),
            sector: "Energy".into(),
            composite_score: 72.0,
            rank_label: "TOP_QUARTILE".into(),
            ..Default::default()
        };
        upsert_momf(&conn, "MMF", &snap).unwrap();
        let got = get_momf(&conn, "MMF").unwrap().unwrap();
        assert_eq!(got.rank_label, "TOP_QUARTILE");
    }

    #[test]
    fn peadrank_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v17(&conn).expect("create v17");
        let snap = PeadRankSnapshot {
            symbol: "PRK".into(),
            as_of: "2026-04-15".into(),
            sector: "Healthcare".into(),
            avg_drift_5d_pct: 2.1,
            rank_label: "ABOVE_MEDIAN".into(),
            ..Default::default()
        };
        upsert_peadrank(&conn, "PRK", &snap).unwrap();
        let got = get_peadrank(&conn, "PRK").unwrap().unwrap();
        assert_eq!(got.rank_label, "ABOVE_MEDIAN");
    }

    #[test]
    fn fqm_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v17(&conn).expect("create v17");
        let snap = FundamentalQualityMeterSnapshot {
            symbol: "FQM".into(),
            as_of: "2026-04-15".into(),
            piotroski_score: 8,
            composite_score: 88.0,
            operator_label: "ELITE_OPERATOR".into(),
            inputs_available: 3,
            ..Default::default()
        };
        upsert_fqm(&conn, "FQM", &snap).unwrap();
        let got = get_fqm(&conn, "FQM").unwrap().unwrap();
        assert_eq!(got.operator_label, "ELITE_OPERATOR");
    }

    #[test]
    fn revrank_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v17(&conn).expect("create v17");
        let snap = RevenueGrowthRankSnapshot {
            symbol: "RVR".into(),
            as_of: "2026-04-15".into(),
            sector: "Technology".into(),
            symbol_cagr_pct: 25.0,
            sector_median_cagr_pct: 10.0,
            gap_to_median_pp: 15.0,
            relative_label: "FAR_ABOVE".into(),
            ..Default::default()
        };
        upsert_revrank(&conn, "RVR", &snap).unwrap();
        let got = get_revrank(&conn, "RVR").unwrap().unwrap();
        assert_eq!(got.relative_label, "FAR_ABOVE");
    }

    #[test]
    fn compute_sizef_top_decile() {
        let peers: Vec<(String, f64)> = vec![
            ("A".into(), 1e9),
            ("B".into(), 2e9),
            ("C".into(), 5e9),
            ("D".into(), 3e9),
        ];
        let snap = compute_sizef_snapshot("MEGA", "2026-04-15", "Technology", Some(5e11), &peers);
        assert_eq!(snap.tier_label, "MEGA_CAP");
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert_eq!(snap.peers_considered, 4);
    }

    #[test]
    fn compute_sizef_no_subject() {
        let snap = compute_sizef_snapshot("NIL", "2026-04-15", "Technology", None, &[]);
        assert_eq!(snap.tier_label, "NO_DATA");
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_momf_above_median() {
        let peers_owned = [mk_mom("A", 40.0), mk_mom("B", 50.0), mk_mom("C", 60.0), mk_mom("D", 70.0)];
        let peers: Vec<&MomentumSnapshot> = peers_owned.iter().collect();
        let subj = mk_mom("MMF", 65.0);
        let snap = compute_momf_snapshot("MMF", "2026-04-15", "Energy", Some(&subj), &peers);
        assert!(snap.percentile_rank > 50.0);
        assert_ne!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_momf_no_subject() {
        let snap = compute_momf_snapshot("N", "2026-04-15", "S", None, &[]);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_peadrank_above_median() {
        let peers_owned = [
            mk_pead("A", 0.5, 4),
            mk_pead("B", 1.0, 5),
            mk_pead("C", 1.5, 4),
            mk_pead("D", 2.0, 4),
        ];
        let peers: Vec<&PeadSnapshot> = peers_owned.iter().collect();
        let subj = mk_pead("PRK", 1.8, 5);
        let snap = compute_peadrank_snapshot("PRK", "2026-04-15", "Healthcare", Some(&subj), &peers);
        assert!(snap.percentile_rank > 50.0);
        assert_ne!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_peadrank_insufficient() {
        let snap = compute_peadrank_snapshot("N", "2026-04-15", "S", None, &[]);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_fqm_elite_operator() {
        let p = mk_ptfs("FQM", 9);
        let m = mk_margins_q("FQM", 35.0, "HIGH", "EXPANDING");
        let a = mk_accruals("FQM", 115.0, "IMPROVING");
        let snap = compute_fqm_snapshot("FQM", "2026-04-15", Some(&p), Some(&m), Some(&a));
        assert_eq!(snap.operator_label, "ELITE_OPERATOR");
        assert_eq!(snap.inputs_available, 3);
        assert!(snap.composite_score >= 85.0);
    }

    #[test]
    fn compute_fqm_no_inputs() {
        let snap = compute_fqm_snapshot("NIL", "2026-04-15", None, None, None);
        assert_eq!(snap.operator_label, "NO_DATA");
        assert_eq!(snap.inputs_available, 0);
    }

    #[test]
    fn compute_revrank_far_above() {
        let subj = mk_financials_with_revenue("RVR", &[2000.0, 1600.0, 1300.0, 1000.0]);
        let peer_stmts: Vec<(String, FinancialStatements)> = vec![
            ("A".into(), mk_financials_with_revenue("A", &[1100.0, 1080.0, 1050.0, 1000.0])),
            ("B".into(), mk_financials_with_revenue("B", &[1080.0, 1060.0, 1030.0, 1000.0])),
            ("C".into(), mk_financials_with_revenue("C", &[1050.0, 1030.0, 1020.0, 1000.0])),
            ("D".into(), mk_financials_with_revenue("D", &[1070.0, 1050.0, 1030.0, 1000.0])),
        ];
        let snap = compute_revrank_snapshot("RVR", "2026-04-15", "Technology", Some(&subj), &peer_stmts);
        assert_eq!(snap.relative_label, "FAR_ABOVE");
        assert!(snap.symbol_cagr_pct > 20.0);
        assert!(snap.gap_to_median_pp > 15.0);
    }

    #[test]
    fn compute_revrank_insufficient_subject() {
        let snap = compute_revrank_snapshot("NIL", "2026-04-15", "S", None, &[]);
        assert_eq!(snap.relative_label, "NO_DATA");
    }

    // ── ADR-125 Round 18 tests ────────────────────────────────────────────

    fn mk_lev(sym: &str, debt: f64, equity: f64) -> LeverageSnapshot {
        LeverageSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            total_debt: debt,
            total_equity: equity,
            ..Default::default()
        }
    }

    fn mk_margins_op(sym: &str, op_margin: f64) -> MarginsSnapshot {
        MarginsSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            latest_operating_margin_pct: op_margin,
            overall_trend_label: "STABLE".into(),
            periods_used: 4,
            ..Default::default()
        }
    }

    fn mk_fqm(sym: &str, composite: f64, operator: &str) -> FundamentalQualityMeterSnapshot {
        FundamentalQualityMeterSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            composite_score: composite,
            operator_label: operator.into(),
            inputs_available: 3,
            ..Default::default()
        }
    }

    fn mk_liq(sym: &str, adv_dollar: f64, tier: &str) -> LiquiditySnapshot {
        LiquiditySnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            avg_daily_dollar_volume: adv_dollar,
            liquidity_tier: tier.into(),
            ..Default::default()
        }
    }

    fn mk_surp(date: &str, surprise_pct: f64) -> EarningsSurprise {
        EarningsSurprise {
            date: date.into(),
            symbol: "X".into(),
            eps_actual: 1.0 + surprise_pct / 100.0,
            eps_estimate: 1.0,
            surprise: surprise_pct / 100.0,
            surprise_pct,
        }
    }

    #[test]
    fn levrank_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v18(&conn).expect("create v18");
        let snap = LeverageRankSnapshot {
            symbol: "LVR".into(),
            as_of: "2026-04-15".into(),
            sector: "Industrials".into(),
            debt_to_equity: 0.3,
            rank_label: "SAFEST_QUARTILE".into(),
            ..Default::default()
        };
        upsert_levrank(&conn, "LVR", &snap).unwrap();
        let got = get_levrank(&conn, "LVR").unwrap().unwrap();
        assert_eq!(got.rank_label, "SAFEST_QUARTILE");
        assert!((got.debt_to_equity - 0.3).abs() < 1e-9);
    }

    #[test]
    fn operank_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v18(&conn).expect("create v18");
        let snap = OperatingQualityRankSnapshot {
            symbol: "OPR".into(),
            as_of: "2026-04-15".into(),
            sector: "Technology".into(),
            operating_margin_pct: 35.0,
            rank_label: "TOP_DECILE".into(),
            ..Default::default()
        };
        upsert_operank(&conn, "OPR", &snap).unwrap();
        let got = get_operank(&conn, "OPR").unwrap().unwrap();
        assert_eq!(got.rank_label, "TOP_DECILE");
    }

    #[test]
    fn fqmrank_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v18(&conn).expect("create v18");
        let snap = FqmRankSnapshot {
            symbol: "FQR".into(),
            as_of: "2026-04-15".into(),
            sector: "Healthcare".into(),
            composite_score: 85.0,
            operator_label: "ELITE_OPERATOR".into(),
            rank_label: "TOP_DECILE".into(),
            ..Default::default()
        };
        upsert_fqmrank(&conn, "FQR", &snap).unwrap();
        let got = get_fqmrank(&conn, "FQR").unwrap().unwrap();
        assert_eq!(got.operator_label, "ELITE_OPERATOR");
        assert_eq!(got.rank_label, "TOP_DECILE");
    }

    #[test]
    fn liqrank_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v18(&conn).expect("create v18");
        let snap = LiquidityRankSnapshot {
            symbol: "LQR".into(),
            as_of: "2026-04-15".into(),
            sector: "Financials".into(),
            avg_daily_dollar_volume: 2.5e9,
            tier_label: "DEEP".into(),
            rank_label: "TOP_QUARTILE".into(),
            ..Default::default()
        };
        upsert_liqrank(&conn, "LQR", &snap).unwrap();
        let got = get_liqrank(&conn, "LQR").unwrap().unwrap();
        assert_eq!(got.tier_label, "DEEP");
        assert_eq!(got.rank_label, "TOP_QUARTILE");
    }

    #[test]
    fn surpstk_snapshot_roundtrip() {
        let conn = Connection::open_in_memory().expect("open conn");
        create_research_tables_v18(&conn).expect("create v18");
        let snap = EarningsSurpriseStreakSnapshot {
            symbol: "SUR".into(),
            as_of: "2026-04-15".into(),
            total_events: 8,
            beats: 7,
            misses: 1,
            beat_rate_pct: 87.5,
            current_streak_type: "BEAT".into(),
            current_streak_len: 5,
            streak_label: "HOT_STREAK".into(),
            ..Default::default()
        };
        upsert_surpstk(&conn, "SUR", &snap).unwrap();
        let got = get_surpstk(&conn, "SUR").unwrap().unwrap();
        assert_eq!(got.streak_label, "HOT_STREAK");
        assert_eq!(got.beats, 7);
    }

    #[test]
    fn compute_levrank_safest_decile() {
        // Subject has the LOWEST D/E in sector → should rank safest.
        let peers_owned = [
            mk_lev("A", 500.0, 1000.0), // D/E 0.50
            mk_lev("B", 800.0, 1000.0), // D/E 0.80
            mk_lev("C", 1200.0, 1000.0),// D/E 1.20
            mk_lev("D", 1500.0, 1000.0),// D/E 1.50
        ];
        let peers: Vec<&LeverageSnapshot> = peers_owned.iter().collect();
        let subj = mk_lev("LVR", 100.0, 1000.0); // D/E 0.10
        let snap = compute_levrank_snapshot("LVR", "2026-04-15", "Industrials", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "SAFEST_DECILE");
        assert!(snap.percentile_rank >= 90.0);
        assert_eq!(snap.rank_position, 1);
    }

    #[test]
    fn compute_levrank_negative_equity() {
        let subj = mk_lev("NEG", 500.0, -100.0);
        let snap = compute_levrank_snapshot("NEG", "2026-04-15", "S", Some(&subj), &[]);
        assert_eq!(snap.rank_label, "NEGATIVE_EQUITY");
    }

    #[test]
    fn compute_levrank_no_subject() {
        let snap = compute_levrank_snapshot("NIL", "2026-04-15", "S", None, &[]);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_operank_top_decile() {
        let peers_owned = [
            mk_margins_op("A", 5.0),
            mk_margins_op("B", 10.0),
            mk_margins_op("C", 15.0),
            mk_margins_op("D", 20.0),
        ];
        let peers: Vec<&MarginsSnapshot> = peers_owned.iter().collect();
        let subj = mk_margins_op("OPR", 45.0);
        let snap = compute_operank_snapshot("OPR", "2026-04-15", "Technology", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert!(snap.percentile_rank >= 90.0);
    }

    #[test]
    fn compute_operank_no_subject() {
        let snap = compute_operank_snapshot("NIL", "2026-04-15", "S", None, &[]);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_fqmrank_top_decile() {
        let peers_owned = [
            mk_fqm("A", 40.0, "WEAK_OPERATOR"),
            mk_fqm("B", 55.0, "AVERAGE_OPERATOR"),
            mk_fqm("C", 65.0, "STRONG_OPERATOR"),
            mk_fqm("D", 72.0, "STRONG_OPERATOR"),
        ];
        let peers: Vec<&FundamentalQualityMeterSnapshot> = peers_owned.iter().collect();
        let subj = mk_fqm("FQR", 92.0, "ELITE_OPERATOR");
        let snap = compute_fqmrank_snapshot("FQR", "2026-04-15", "Technology", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert_eq!(snap.operator_label, "ELITE_OPERATOR");
    }

    #[test]
    fn compute_fqmrank_filters_no_data_peers() {
        let peers_owned = [
            mk_fqm("A", 0.0, "NO_DATA"),
            mk_fqm("B", 0.0, "NO_DATA"),
            mk_fqm("C", 0.0, "NO_DATA"),
            mk_fqm("D", 0.0, "NO_DATA"),
        ];
        let peers: Vec<&FundamentalQualityMeterSnapshot> = peers_owned.iter().collect();
        let subj = mk_fqm("FQR", 90.0, "ELITE_OPERATOR");
        let snap = compute_fqmrank_snapshot("FQR", "2026-04-15", "T", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_liqrank_deepest() {
        let peers_owned = [
            mk_liq("A", 1e6, "THIN"),
            mk_liq("B", 5e7, "MODERATE"),
            mk_liq("C", 2e8, "LIQUID"),
            mk_liq("D", 8e8, "LIQUID"),
        ];
        let peers: Vec<&LiquiditySnapshot> = peers_owned.iter().collect();
        let subj = mk_liq("LQR", 5e9, "DEEP");
        let snap = compute_liqrank_snapshot("LQR", "2026-04-15", "Financials", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert_eq!(snap.rank_position, 1);
        assert_eq!(snap.tier_label, "DEEP");
    }

    #[test]
    fn compute_liqrank_filters_insufficient_data() {
        let peers_owned = [
            mk_liq("A", 0.0, "INSUFFICIENT_DATA"),
            mk_liq("B", 0.0, "INSUFFICIENT_DATA"),
            mk_liq("C", 0.0, "INSUFFICIENT_DATA"),
        ];
        let peers: Vec<&LiquiditySnapshot> = peers_owned.iter().collect();
        let subj = mk_liq("LQR", 1e9, "DEEP");
        let snap = compute_liqrank_snapshot("LQR", "2026-04-15", "Financials", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_surpstk_hot_streak() {
        let rows = vec![
            mk_surp("2026-04-01", 12.0),
            mk_surp("2026-01-01", 9.0),
            mk_surp("2025-10-01", 6.0),
            mk_surp("2025-07-01", 4.0),
            mk_surp("2025-04-01", 5.0),
            mk_surp("2025-01-01", 3.0),
            mk_surp("2024-10-01", 1.0),
            mk_surp("2024-07-01", -3.0),
        ];
        let snap = compute_surpstk_snapshot("HOT", "2026-04-15", &rows);
        assert_eq!(snap.streak_label, "HOT_STREAK");
        assert_eq!(snap.current_streak_type, "BEAT");
        assert!(snap.current_streak_len >= 3);
        assert!(snap.beat_rate_pct >= 75.0);
    }

    #[test]
    fn compute_surpstk_cold_streak() {
        let rows = vec![
            mk_surp("2026-04-01", -8.0),
            mk_surp("2026-01-01", -5.0),
            mk_surp("2025-10-01", -4.0),
            mk_surp("2025-07-01", -3.0),
            mk_surp("2025-04-01", -6.0),
            mk_surp("2025-01-01", -2.5),
            mk_surp("2024-10-01", 1.0),
            mk_surp("2024-07-01", 2.5),
        ];
        let snap = compute_surpstk_snapshot("CLD", "2026-04-15", &rows);
        assert_eq!(snap.streak_label, "COLD_STREAK");
        assert_eq!(snap.current_streak_type, "MISS");
        assert!(snap.current_streak_len >= 3);
    }

    #[test]
    fn compute_surpstk_mixed() {
        // 50% beat rate, alternating → neither BEAT_TREND nor MISS_TREND.
        let rows = vec![
            mk_surp("2026-04-01", 3.0),
            mk_surp("2026-01-01", -3.0),
            mk_surp("2025-10-01", 3.0),
            mk_surp("2025-07-01", -3.0),
            mk_surp("2025-04-01", 3.0),
            mk_surp("2025-01-01", -3.0),
        ];
        let snap = compute_surpstk_snapshot("MIX", "2026-04-15", &rows);
        assert_eq!(snap.streak_label, "MIXED");
        assert_eq!(snap.beats, 3);
        assert_eq!(snap.misses, 3);
    }

    #[test]
    fn compute_surpstk_insufficient_data() {
        let snap = compute_surpstk_snapshot("NIL", "2026-04-15", &[]);
        assert_eq!(snap.streak_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-126 Round 19 tests ────────────────────────────────────────────

    fn mk_divg(sym: &str, cagr3: f64, trend: &str) -> DivgSnapshot {
        DivgSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            total_payments: 12,
            years_covered: 10,
            cagr_3y_pct: cagr3,
            consecutive_growth_years: 5,
            trend_label: trend.into(),
            ..Default::default()
        }
    }

    fn mk_earm(sym: &str, score: f64, label: &str) -> EarmSnapshot {
        EarmSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            quarters_used: 8,
            composite_score: score,
            momentum_label: label.into(),
            ..Default::default()
        }
    }

    fn mk_updm(sym: &str, net90: i32, bias: &str) -> UpdmSnapshot {
        UpdmSnapshot {
            symbol: sym.into(),
            as_of: "2026-04-15".into(),
            total_actions: 10,
            net_90d: net90,
            bias_label: bias.into(),
            ..Default::default()
        }
    }

    fn mk_hp(date: &str, open: f64, high: f64, low: f64, close: f64) -> HistoricalPriceRow {
        HistoricalPriceRow {
            date: date.into(),
            open,
            high,
            low,
            close,
            adj_close: close,
            volume: 1_000_000.0,
            change: close - open,
            change_pct: 0.0,
        }
    }

    #[test]
    fn dvdrank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v19(&c).unwrap();
        let snap = DividendGrowthRankSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            sector: "Tech".into(),
            cagr_3y_pct: 12.5,
            rank_label: "TOP_DECILE".into(),
            ..Default::default()
        };
        upsert_dvdrank(&c, "AAA", &snap).unwrap();
        let got = get_dvdrank(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.rank_label, "TOP_DECILE");
    }

    #[test]
    fn earmrank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v19(&c).unwrap();
        let snap = EarningsMomentumRankSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            sector: "Tech".into(),
            composite_score: 72.0,
            rank_label: "ABOVE_MEDIAN".into(),
            ..Default::default()
        };
        upsert_earmrank(&c, "AAA", &snap).unwrap();
        let got = get_earmrank(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.rank_label, "ABOVE_MEDIAN");
    }

    #[test]
    fn updgrank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v19(&c).unwrap();
        let snap = UpgradeDowngradeRankSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            sector: "Tech".into(),
            net_90d: 5,
            rank_label: "BELOW_MEDIAN".into(),
            ..Default::default()
        };
        upsert_updgrank(&c, "AAA", &snap).unwrap();
        let got = get_updgrank(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.net_90d, 5);
    }

    #[test]
    fn gy_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v19(&c).unwrap();
        let snap = GapYearlySnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            gaps_total: 8,
            gap_label: "NORMAL".into(),
            ..Default::default()
        };
        upsert_gy(&c, "AAA", &snap).unwrap();
        let got = get_gy(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.gap_label, "NORMAL");
    }

    #[test]
    fn des_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v19(&c).unwrap();
        let snap = DailyEventStreakSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            bars_used: 252,
            current_streak_type: "UP".into(),
            current_streak_len: 3,
            streak_label: "STRONG_UPTREND".into(),
            ..Default::default()
        };
        upsert_des(&c, "AAA", &snap).unwrap();
        let got = get_des(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.streak_label, "STRONG_UPTREND");
    }

    #[test]
    fn compute_dvdrank_top_decile() {
        let subj = mk_divg("AAA", 15.0, "GROWING");
        let p1 = mk_divg("BBB", 3.0, "GROWING");
        let p2 = mk_divg("CCC", 5.0, "GROWING");
        let p3 = mk_divg("DDD", 2.0, "STABLE");
        let p4 = mk_divg("EEE", 1.0, "GROWING");
        let peers = vec![&p1, &p2, &p3, &p4];
        let snap = compute_dvdrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert!(snap.percentile_rank >= 90.0);
    }

    #[test]
    fn compute_dvdrank_no_history_filtered() {
        let subj = mk_divg("AAA", 5.0, "GROWING");
        let bad = mk_divg("BBB", 0.0, "NO_HISTORY");
        let ok1 = mk_divg("CCC", 3.0, "STABLE");
        let ok2 = mk_divg("DDD", 2.0, "GROWING");
        let ok3 = mk_divg("EEE", 4.0, "STABLE");
        let peers = vec![&bad, &ok1, &ok2, &ok3];
        let snap = compute_dvdrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.peers_considered, 4);
        assert_eq!(snap.peers_with_data, 3);
        assert!(snap.rank_label != "INSUFFICIENT_DATA");
        assert!(snap.rank_label != "NO_DATA");
    }

    #[test]
    fn compute_earmrank_above_median() {
        let subj = mk_earm("AAA", 75.0, "ACCELERATING");
        let p1 = mk_earm("BBB", 40.0, "STABLE");
        let p2 = mk_earm("CCC", 50.0, "STABLE");
        let p3 = mk_earm("DDD", 60.0, "ACCELERATING");
        let p4 = mk_earm("EEE", 30.0, "DECELERATING");
        let peers = vec![&p1, &p2, &p3, &p4];
        let snap = compute_earmrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
        assert!(snap.percentile_rank > 50.0);
        assert_eq!(snap.composite_score, 75.0);
    }

    #[test]
    fn compute_earmrank_insufficient_filtered() {
        let subj = mk_earm("AAA", 65.0, "STABLE");
        let bad = mk_earm("BBB", 0.0, "INSUFFICIENT_DATA");
        let peers = vec![&bad];
        let snap = compute_earmrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.rank_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_updgrank_bullish() {
        let subj = mk_updm("AAA", 8, "BULLISH");
        let p1 = mk_updm("BBB", -2, "BEARISH");
        let p2 = mk_updm("CCC", 1, "NEUTRAL");
        let p3 = mk_updm("DDD", 3, "BULLISH");
        let p4 = mk_updm("EEE", -5, "BEARISH");
        let peers = vec![&p1, &p2, &p3, &p4];
        let snap = compute_updgrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.net_90d, 8);
        assert!(snap.percentile_rank > 60.0);
    }

    #[test]
    fn compute_updgrank_no_coverage_filtered() {
        let subj = mk_updm("AAA", 3, "BULLISH");
        let bad = mk_updm("BBB", 0, "NO_COVERAGE");
        let ok = mk_updm("CCC", 0, "NEUTRAL");
        let peers = vec![&bad, &ok];
        let snap = compute_updgrank_snapshot("AAA", "2026-04-15", "Tech", Some(&subj), &peers);
        assert_eq!(snap.peers_with_data, 1);
    }

    #[test]
    fn compute_gy_normal() {
        // 30-bar window, one small up gap, one small down gap, rest flat.
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-01-{:02}", i + 1);
            // Day 5: prev close 100, today open 102.5 → +2.5% gap
            // Day 10: prev close 100, today open 97.5 → -2.5% gap
            let open = if i == 5 { 102.5 } else if i == 10 { 97.5 } else { 100.0 };
            bars.push(mk_hp(&date, open, open + 1.0, open - 1.0, 100.0));
        }
        let snap = compute_gy_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.bars_used >= 20);
        assert!(snap.gaps_up_2pct >= 1);
        assert!(snap.gaps_down_2pct >= 1);
    }

    #[test]
    fn compute_gy_explosive() {
        // 30-bar window with a single 12% gap up → EXPLOSIVE via 10% band.
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-02-{:02}", i + 1);
            let open = if i == 15 { 112.0 } else { 100.0 };
            bars.push(mk_hp(&date, open, open + 1.0, open - 1.0, 100.0));
        }
        let snap = compute_gy_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.gap_label, "EXPLOSIVE");
        assert_eq!(snap.gaps_up_10pct, 1);
    }

    #[test]
    fn compute_gy_insufficient() {
        let bars = vec![mk_hp("2025-03-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_gy_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.gap_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_des_uptrend() {
        // 30 bars, strictly rising close each day → STRONG_UPTREND.
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-04-{:02}", i + 1);
            let close = 100.0 + i as f64;
            bars.push(mk_hp(&date, close - 0.5, close + 0.5, close - 0.5, close));
        }
        let snap = compute_des_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.streak_label, "STRONG_UPTREND");
        assert_eq!(snap.current_streak_type, "UP");
        assert!(snap.longest_up_streak >= 5);
    }

    #[test]
    fn compute_des_downtrend() {
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-05-{:02}", i + 1);
            let close = 200.0 - i as f64;
            bars.push(mk_hp(&date, close + 0.5, close + 0.5, close - 0.5, close));
        }
        let snap = compute_des_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.streak_label, "STRONG_DOWNTREND");
        assert_eq!(snap.current_streak_type, "DOWN");
    }

    #[test]
    fn compute_des_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_des_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.streak_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-127 Round 20 tests ────────────────────────────────────────────

    #[test]
    fn dvdyieldrank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v20(&c).unwrap();
        let snap = DividendYieldRankSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            sector: "Utilities".into(),
            dividend_yield_pct: 4.5,
            rank_label: "TOP_DECILE".into(),
            ..Default::default()
        };
        upsert_dvdyieldrank(&c, "AAA", &snap).unwrap();
        let got = get_dvdyieldrank(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.rank_label, "TOP_DECILE");
        assert_eq!(got.dividend_yield_pct, 4.5);
    }

    #[test]
    fn shrank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v20(&c).unwrap();
        let snap = ShortInterestRankSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            sector: "Tech".into(),
            short_pct_of_float: 2.5,
            rank_label: "SAFEST_DECILE".into(),
            ..Default::default()
        };
        upsert_shrank(&c, "AAA", &snap).unwrap();
        let got = get_shrank(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.rank_label, "SAFEST_DECILE");
        assert_eq!(got.short_pct_of_float, 2.5);
    }

    #[test]
    fn atrann_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v20(&c).unwrap();
        let snap = AnnualizedAtrSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            latest_close: 100.0,
            atr14: 1.5,
            atr14_pct: 1.5,
            atr_annualized_pct: 23.8,
            regime_label: "NORMAL_VOL".into(),
            ..Default::default()
        };
        upsert_atrann(&c, "AAA", &snap).unwrap();
        let got = get_atrann(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.regime_label, "NORMAL_VOL");
        assert!((got.atr_annualized_pct - 23.8).abs() < 1e-9);
    }

    #[test]
    fn ddhist_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v20(&c).unwrap();
        let snap = DrawdownHistorySnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            max_drawdown_pct: -12.0,
            longest_drawdown_days: 45,
            corrections_5pct: 3,
            corrections_10pct: 1,
            current_drawdown_pct: -2.0,
            regime_label: "MEANINGFUL".into(),
            ..Default::default()
        };
        upsert_ddhist(&c, "AAA", &snap).unwrap();
        let got = get_ddhist(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.regime_label, "MEANINGFUL");
        assert_eq!(got.corrections_5pct, 3);
    }

    #[test]
    fn priceperf_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v20(&c).unwrap();
        let snap = PricePerformanceSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            latest_close: 120.0,
            ret_1m_pct: 2.5,
            ret_3m_pct: 8.0,
            ret_6m_pct: 12.0,
            ret_ytd_pct: 15.0,
            ret_1y_pct: 20.0,
            trend_label: "BULL".into(),
            ..Default::default()
        };
        upsert_priceperf(&c, "AAA", &snap).unwrap();
        let got = get_priceperf(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.trend_label, "BULL");
        assert!((got.ret_1y_pct - 20.0).abs() < 1e-9);
    }

    #[test]
    fn compute_dvdyieldrank_top_decile() {
        let peers = vec![
            ("BBB".to_string(), Some(1.5)),
            ("CCC".to_string(), Some(2.0)),
            ("DDD".to_string(), Some(2.5)),
            ("EEE".to_string(), Some(1.0)),
        ];
        let snap = compute_dvdyieldrank_snapshot("AAA", "2026-04-15", "Utilities", Some(6.0), &peers);
        assert!(snap.percentile_rank >= 90.0);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert_eq!(snap.peers_with_data, 4);
    }

    #[test]
    fn compute_dvdyieldrank_non_payer_filtered() {
        let peers = vec![
            ("BBB".to_string(), None),              // non-payer
            ("CCC".to_string(), Some(0.0)),         // non-payer (zero yield)
            ("DDD".to_string(), Some(3.0)),
            ("EEE".to_string(), Some(4.0)),
            ("FFF".to_string(), Some(2.0)),
        ];
        let snap = compute_dvdyieldrank_snapshot("AAA", "2026-04-15", "Utilities", Some(5.0), &peers);
        assert_eq!(snap.peers_considered, 5);
        assert_eq!(snap.peers_with_data, 3);
        assert_ne!(snap.rank_label, "INSUFFICIENT_DATA");
        assert_ne!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_dvdyieldrank_subject_non_payer() {
        let peers = vec![
            ("BBB".to_string(), Some(3.0)),
            ("CCC".to_string(), Some(4.0)),
            ("DDD".to_string(), Some(2.0)),
        ];
        let snap = compute_dvdyieldrank_snapshot("AAA", "2026-04-15", "Tech", Some(0.0), &peers);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_shrank_safest_decile() {
        // Subject has lowest short interest → risk-inverted top rank (SAFEST).
        let peers = vec![
            ("BBB".to_string(), Some(8.0)),
            ("CCC".to_string(), Some(10.0)),
            ("DDD".to_string(), Some(12.0)),
            ("EEE".to_string(), Some(15.0)),
        ];
        let snap = compute_shrank_snapshot("AAA", "2026-04-15", "Tech", Some(1.0), &peers);
        assert!(snap.percentile_rank >= 90.0);
        assert_eq!(snap.rank_label, "SAFEST_DECILE");
        assert_eq!(snap.short_pct_of_float, 1.0);
    }

    #[test]
    fn compute_shrank_riskiest_decile() {
        // Subject has highest short interest → risk-inverted bottom rank (RISKIEST).
        // Need ≥10 peers so the floor 0.5/total*100 is strictly below 10.
        let peers = vec![
            ("BBB".to_string(), Some(2.0)),
            ("CCC".to_string(), Some(3.0)),
            ("DDD".to_string(), Some(4.0)),
            ("EEE".to_string(), Some(5.0)),
            ("FFF".to_string(), Some(6.0)),
            ("GGG".to_string(), Some(7.0)),
            ("HHH".to_string(), Some(8.0)),
            ("III".to_string(), Some(9.0)),
            ("JJJ".to_string(), Some(10.0)),
            ("KKK".to_string(), Some(11.0)),
        ];
        let snap = compute_shrank_snapshot("AAA", "2026-04-15", "Tech", Some(25.0), &peers);
        assert!(snap.percentile_rank < 10.0);
        assert_eq!(snap.rank_label, "RISKIEST_DECILE");
    }

    #[test]
    fn compute_shrank_insufficient() {
        let peers = vec![
            ("BBB".to_string(), None),
            ("CCC".to_string(), Some(5.0)),
        ];
        let snap = compute_shrank_snapshot("AAA", "2026-04-15", "Tech", Some(3.0), &peers);
        assert_eq!(snap.rank_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_atrann_low_vol() {
        // 30 quiet bars (≤0.5% HL range) → LOW_VOL regime.
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-01-{:02}", i + 1);
            bars.push(mk_hp(&date, 100.0, 100.3, 99.7, 100.0));
        }
        let snap = compute_atrann_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.regime_label, "LOW_VOL");
        assert!(snap.atr_annualized_pct < 15.0);
    }

    #[test]
    fn compute_atrann_high_vol() {
        // 30 wild bars (5% HL range) → HIGH_VOL or EXTREME_VOL.
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-02-{:02}", i + 1);
            bars.push(mk_hp(&date, 100.0, 103.0, 97.0, 100.0));
        }
        let snap = compute_atrann_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.atr_annualized_pct > 30.0, "expected > 30% annualized, got {}", snap.atr_annualized_pct);
        assert!(snap.regime_label == "HIGH_VOL" || snap.regime_label == "EXTREME_VOL");
    }

    #[test]
    fn compute_atrann_insufficient() {
        let bars = vec![mk_hp("2025-03-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_atrann_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_ddhist_shallow() {
        // 30 quiet bars, max 3% dip → SHALLOW regime.
        let mut bars = Vec::new();
        for i in 0..30 {
            let date = format!("2025-04-{:02}", i + 1);
            let c = if (5..15).contains(&i) { 98.0 } else { 100.0 };
            bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
        }
        let snap = compute_ddhist_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.max_drawdown_pct > -5.0);
        assert!(snap.regime_label == "SHALLOW" || snap.regime_label == "RECOVERING");
    }

    #[test]
    fn compute_ddhist_severe() {
        // Rise to peak, then 25% decline and partial recovery → SEVERE.
        let mut bars = Vec::new();
        for i in 0..20 {
            let date = format!("2025-05-{:02}", i + 1);
            bars.push(mk_hp(&date, 100.0, 101.0, 99.0, 100.0));
        }
        // Peak at 120
        for i in 0..10 {
            let date = format!("2025-06-{:02}", i + 1);
            let c = 100.0 + (i as f64 + 1.0) * 2.0;
            bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
        }
        // Crash down 25% to 90
        for i in 0..10 {
            let date = format!("2025-07-{:02}", i + 1);
            let c = 120.0 - (i as f64 + 1.0) * 3.0;
            bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
        }
        let snap = compute_ddhist_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.max_drawdown_pct < -20.0, "expected < -20%, got {}", snap.max_drawdown_pct);
        assert!(snap.regime_label == "SEVERE" || snap.regime_label == "CATASTROPHIC" || snap.regime_label == "MEANINGFUL");
    }

    #[test]
    fn compute_priceperf_bull() {
        // Sustained 25%+ rally over the window → BULL or STRONG_BULL.
        let mut bars = Vec::new();
        for i in 0..260 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let c = 100.0 + i as f64 * 0.15;  // ~39% rise over window
            bars.push(mk_hp(&date, c, c + 0.2, c - 0.2, c));
        }
        let snap = compute_priceperf_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.ret_1y_pct > 10.0);
        assert!(snap.trend_label == "BULL" || snap.trend_label == "STRONG_BULL");
    }

    #[test]
    fn compute_priceperf_bear() {
        // Sustained decline → BEAR or STRONG_BEAR.
        let mut bars = Vec::new();
        for i in 0..260 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let c = 200.0 - i as f64 * 0.3;  // ~39% decline
            bars.push(mk_hp(&date, c, c + 0.2, c - 0.2, c));
        }
        let snap = compute_priceperf_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.ret_1y_pct < -10.0);
        assert!(snap.trend_label == "BEAR" || snap.trend_label == "STRONG_BEAR");
    }

    #[test]
    fn compute_priceperf_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_priceperf_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.trend_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-128 Round 21 tests ──

    #[test]
    fn betarank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v21(&c).unwrap();
        let snap = BetaRankSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            sector: "Technology".into(),
            subject_beta: Some(0.8),
            percentile_rank: 85.0,
            rank_label: "SAFEST_QUARTILE".into(),
            ..Default::default()
        };
        upsert_betarank(&c, "AAA", &snap).unwrap();
        let got = get_betarank(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.rank_label, "SAFEST_QUARTILE");
        assert!((got.subject_beta.unwrap() - 0.8).abs() < 1e-9);
    }

    #[test]
    fn pegrank_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v21(&c).unwrap();
        let snap = PegRankSnapshot {
            symbol: "BBB".into(),
            as_of: "2026-04-15".into(),
            sector: "Technology".into(),
            subject_peg: Some(0.9),
            percentile_rank: 90.0,
            rank_label: "TOP_DECILE".into(),
            ..Default::default()
        };
        upsert_pegrank(&c, "BBB", &snap).unwrap();
        let got = get_pegrank(&c, "BBB").unwrap().unwrap();
        assert_eq!(got.rank_label, "TOP_DECILE");
    }

    #[test]
    fn fhighlow_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v21(&c).unwrap();
        let snap = FiftyTwoWeekHighLowSnapshot {
            symbol: "CCC".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            latest_close: 120.0,
            high_52w: 150.0,
            high_52w_date: "2025-11-01".into(),
            days_since_high: 100,
            low_52w: 80.0,
            low_52w_date: "2025-07-01".into(),
            days_since_low: 200,
            pct_from_high: -20.0,
            pct_from_low: 50.0,
            range_position_pct: 57.0,
            proximity_label: "MID_RANGE".into(),
            ..Default::default()
        };
        upsert_fhighlow(&c, "CCC", &snap).unwrap();
        let got = get_fhighlow(&c, "CCC").unwrap().unwrap();
        assert_eq!(got.proximity_label, "MID_RANGE");
    }

    #[test]
    fn rvcone_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v21(&c).unwrap();
        let snap = RealizedVolConeSnapshot {
            symbol: "DDD".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            latest_close: 120.0,
            rv20_pct: 25.0,
            rv60_pct: 22.0,
            rv120_pct: 20.0,
            rv252_pct: 18.0,
            rv20_min_pct: 10.0,
            rv20_median_pct: 20.0,
            rv20_max_pct: 40.0,
            rv20_percentile: 75.0,
            cone_label: "ELEVATED".into(),
            ..Default::default()
        };
        upsert_rvcone(&c, "DDD", &snap).unwrap();
        let got = get_rvcone(&c, "DDD").unwrap().unwrap();
        assert_eq!(got.cone_label, "ELEVATED");
    }

    #[test]
    fn calpb_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v21(&c).unwrap();
        let snap = CalendarPeriodBreakdownSnapshot {
            symbol: "EEE".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            latest_close: 100.0,
            mtd_pct: 2.0,
            qtd_pct: 5.0,
            ytd_pct: 8.0,
            prior_quarter_pct: 1.0,
            prior_year_pct: 10.0,
            current_year: "2026".into(),
            current_quarter: "Q2".into(),
            momentum_label: "ACCELERATING".into(),
            ..Default::default()
        };
        upsert_calpb(&c, "EEE", &snap).unwrap();
        let got = get_calpb(&c, "EEE").unwrap().unwrap();
        assert_eq!(got.momentum_label, "ACCELERATING");
    }

    #[test]
    fn compute_betarank_safest_decile() {
        // Subject has the lowest beta by far → safest decile.
        let peers = vec![
            ("B".to_string(), Some(1.5)),
            ("C".to_string(), Some(1.8)),
            ("D".to_string(), Some(2.0)),
            ("E".to_string(), Some(1.6)),
            ("F".to_string(), Some(1.7)),
            ("G".to_string(), Some(1.9)),
            ("H".to_string(), Some(2.1)),
            ("I".to_string(), Some(1.4)),
            ("J".to_string(), Some(1.55)),
            ("K".to_string(), Some(2.2)),
        ];
        let snap = compute_betarank_snapshot("AAA", "2026-04-15", "Technology", Some(0.6), &peers);
        assert_eq!(snap.rank_label, "SAFEST_DECILE");
        assert_eq!(snap.rank_position, 1);
    }

    #[test]
    fn compute_betarank_riskiest_decile() {
        // Subject has the highest beta → riskiest decile.
        let peers = vec![
            ("B".to_string(), Some(0.6)),
            ("C".to_string(), Some(0.7)),
            ("D".to_string(), Some(0.8)),
            ("E".to_string(), Some(0.9)),
            ("F".to_string(), Some(1.0)),
            ("G".to_string(), Some(1.1)),
            ("H".to_string(), Some(1.2)),
            ("I".to_string(), Some(1.3)),
            ("J".to_string(), Some(1.4)),
            ("K".to_string(), Some(1.5)),
        ];
        let snap = compute_betarank_snapshot("AAA", "2026-04-15", "Technology", Some(2.5), &peers);
        assert_eq!(snap.rank_label, "RISKIEST_DECILE");
    }

    #[test]
    fn compute_betarank_insufficient() {
        let peers = vec![("B".to_string(), Some(1.0)), ("C".to_string(), Some(1.1))];
        let snap = compute_betarank_snapshot("AAA", "2026-04-15", "Technology", Some(0.9), &peers);
        assert_eq!(snap.rank_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_pegrank_top_decile() {
        // Subject has the lowest PEG → top (best value) decile.
        let peers = vec![
            ("B".to_string(), Some(2.0)),
            ("C".to_string(), Some(2.2)),
            ("D".to_string(), Some(2.5)),
            ("E".to_string(), Some(2.8)),
            ("F".to_string(), Some(3.0)),
            ("G".to_string(), Some(3.2)),
            ("H".to_string(), Some(3.5)),
            ("I".to_string(), Some(3.8)),
            ("J".to_string(), Some(4.0)),
            ("K".to_string(), Some(4.5)),
        ];
        let snap = compute_pegrank_snapshot("AAA", "2026-04-15", "Technology", Some(0.5), &peers);
        assert_eq!(snap.rank_label, "TOP_DECILE");
        assert_eq!(snap.rank_position, 1);
    }

    #[test]
    fn compute_pegrank_filters_negative() {
        // Negative/missing peer PEGs get filtered out.
        let peers = vec![
            ("B".to_string(), Some(2.0)),
            ("C".to_string(), Some(2.5)),
            ("D".to_string(), None),
            ("E".to_string(), Some(-1.5)),
            ("F".to_string(), Some(3.0)),
        ];
        let snap = compute_pegrank_snapshot("AAA", "2026-04-15", "Technology", Some(1.5), &peers);
        assert_eq!(snap.peers_with_data, 3);
    }

    #[test]
    fn compute_pegrank_subject_negative() {
        let peers = vec![("B".to_string(), Some(2.0)), ("C".to_string(), Some(2.5)), ("D".to_string(), Some(3.0))];
        let snap = compute_pegrank_snapshot("AAA", "2026-04-15", "Technology", Some(-0.5), &peers);
        assert_eq!(snap.rank_label, "NO_DATA");
    }

    #[test]
    fn compute_fhighlow_at_high() {
        // Latest close == the highest close in the window.
        let mut bars = Vec::new();
        for i in 0..253 {
            let date = format!("2025-{:02}-{:02}", (i / 21) + 1, (i % 21) + 1);
            let c = 100.0 + i as f64 * 0.5;  // monotone up
            bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
        }
        let snap = compute_fhighlow_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.proximity_label, "AT_HIGH");
        assert_eq!(snap.days_since_high, 0);
        assert!((snap.pct_from_high - 0.0).abs() < 1e-9);
    }

    #[test]
    fn compute_fhighlow_at_low() {
        // Monotone down → latest close == lowest close.
        let mut bars = Vec::new();
        for i in 0..253 {
            let date = format!("2025-{:02}-{:02}", (i / 21) + 1, (i % 21) + 1);
            let c = 200.0 - i as f64 * 0.5;
            bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
        }
        let snap = compute_fhighlow_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.proximity_label, "AT_LOW");
        assert_eq!(snap.days_since_low, 0);
    }

    #[test]
    fn compute_fhighlow_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_fhighlow_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.proximity_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_rvcone_compressed() {
        // Flat-ish series → compressed / below-avg realized vol.
        let mut bars = Vec::new();
        for i in 0..260 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let c = 100.0 + (i as f64 * 0.0001);
            bars.push(mk_hp(&date, c, c + 0.001, c - 0.001, c));
        }
        let snap = compute_rvcone_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.cone_label != "INSUFFICIENT_DATA");
        assert!(snap.rv252_pct < 5.0, "expected low vol, got {}", snap.rv252_pct);
    }

    #[test]
    fn compute_rvcone_extreme() {
        // Highly variable → elevated / extreme.
        let mut bars = Vec::new();
        for i in 0..260 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            // Large oscillations with bigger swings at the end.
            let base = 100.0;
            let amp = if i < 200 { 1.0 } else { 10.0 };
            let c = base + amp * ((i as f64 * 0.5).sin() * (i as f64 * 0.3).cos());
            bars.push(mk_hp(&date, c, c + 0.5, c - 0.5, c));
        }
        let snap = compute_rvcone_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.cone_label != "INSUFFICIENT_DATA");
        assert!(snap.rv20_pct > snap.rv252_pct,
            "expected recent 20d RV > 252d RV due to amplitude shift, got rv20={} rv252={}",
            snap.rv20_pct, snap.rv252_pct);
    }

    #[test]
    fn compute_rvcone_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_rvcone_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.cone_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_calpb_accelerating() {
        // Q1 2026 flat, Q2 2026 big up move — accelerating vs prior quarter.
        let mut bars = Vec::new();
        // Prior year 2025 Q4 (Oct-Dec): bars from 100→100 (flat prior year Q4).
        for m in 10..=12 {
            for d in 1..=20 {
                let c = 100.0;
                bars.push(mk_hp(&format!("2025-{:02}-{:02}", m, d), c, c + 0.1, c - 0.1, c));
            }
        }
        // 2026 Q1 (Jan-Mar) flat at 100 → 100.5
        for m in 1..=3 {
            for d in 1..=20 {
                let c = 100.0 + ((m - 1) * 20 + d) as f64 * 0.01;
                bars.push(mk_hp(&format!("2026-{:02}-{:02}", m, d), c, c + 0.1, c - 0.1, c));
            }
        }
        // 2026 Q2 (Apr): big up move.
        for d in 1..=15 {
            let c = 100.5 + d as f64 * 1.0;
            bars.push(mk_hp(&format!("2026-04-{:02}", d), c, c + 0.1, c - 0.1, c));
        }
        let snap = compute_calpb_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.current_year, "2026");
        assert_eq!(snap.current_quarter, "Q2");
        assert!(snap.qtd_pct > 5.0, "expected QTD up, got {}", snap.qtd_pct);
        assert_eq!(snap.momentum_label, "ACCELERATING");
    }

    #[test]
    fn compute_calpb_insufficient() {
        let bars = vec![mk_hp("2026-04-15", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_calpb_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.momentum_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-129 Round 22 tests ──

    #[test]
    fn retskew_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v22(&c).unwrap();
        let snap = ReturnSkewnessSnapshot {
            symbol: "AAA".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            mean_log_return: 0.0005,
            stdev_log_return: 0.012,
            skewness: -0.45,
            positive_return_pct: 52.0,
            largest_up_pct: 4.5,
            largest_down_pct: -6.0,
            skew_label: "LEFT".into(),
            ..Default::default()
        };
        upsert_retskew(&c, "AAA", &snap).unwrap();
        let got = get_retskew(&c, "AAA").unwrap().unwrap();
        assert_eq!(got.skew_label, "LEFT");
        assert!((got.skewness + 0.45).abs() < 1e-9);
    }

    #[test]
    fn retkurt_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v22(&c).unwrap();
        let snap = ReturnKurtosisSnapshot {
            symbol: "BBB".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            excess_kurtosis: 3.5,
            outlier_2sigma_count: 12,
            outlier_3sigma_count: 2,
            outlier_2sigma_pct: 4.74,
            kurt_label: "FAT".into(),
            ..Default::default()
        };
        upsert_retkurt(&c, "BBB", &snap).unwrap();
        let got = get_retkurt(&c, "BBB").unwrap().unwrap();
        assert_eq!(got.kurt_label, "FAT");
    }

    #[test]
    fn tailr_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v22(&c).unwrap();
        let snap = TailRatioSnapshot {
            symbol: "CCC".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            pct_95_return: 2.0,
            pct_05_return: -2.5,
            tail_ratio: 0.8,
            bias_label: "SLIGHT_DOWNSIDE".into(),
            ..Default::default()
        };
        upsert_tailr(&c, "CCC", &snap).unwrap();
        let got = get_tailr(&c, "CCC").unwrap().unwrap();
        assert_eq!(got.bias_label, "SLIGHT_DOWNSIDE");
    }

    #[test]
    fn runlen_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v22(&c).unwrap();
        let snap = RunLengthSnapshot {
            symbol: "DDD".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            avg_up_run: 1.8,
            avg_down_run: 1.6,
            longest_up_run: 5,
            longest_down_run: 4,
            trend_label: "MIXED".into(),
            ..Default::default()
        };
        upsert_runlen(&c, "DDD", &snap).unwrap();
        let got = get_runlen(&c, "DDD").unwrap().unwrap();
        assert_eq!(got.trend_label, "MIXED");
    }

    #[test]
    fn dayrange_snapshot_roundtrip() {
        let c = rusqlite::Connection::open_in_memory().unwrap();
        create_research_tables_v22(&c).unwrap();
        let snap = DailyRangeSnapshot {
            symbol: "EEE".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            avg_range_60_pct: 1.2,
            avg_range_252_pct: 1.5,
            compression_ratio: 0.8,
            range_label: "COMPRESSED".into(),
            ..Default::default()
        };
        upsert_dayrange(&c, "EEE", &snap).unwrap();
        let got = get_dayrange(&c, "EEE").unwrap().unwrap();
        assert_eq!(got.range_label, "COMPRESSED");
    }

    #[test]
    fn compute_retskew_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_retskew_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.skew_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_retskew_left_tail() {
        // Series with mostly small up-days and a few large down-days → negative skew.
        let mut bars = Vec::new();
        let mut c = 100.0;
        bars.push(mk_hp("2025-01-01", c, c + 0.1, c - 0.1, c));
        for i in 0..200 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let change = if i % 25 == 0 { -8.0 } else { 0.3 }; // occasional large down
            c *= 1.0 + change / 100.0;
            bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
        }
        let snap = compute_retskew_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.skew_label != "INSUFFICIENT_DATA");
        assert!(snap.skewness < -0.2, "expected negative skew, got {}", snap.skewness);
    }

    #[test]
    fn compute_retkurt_fat_tails() {
        // Mostly quiet with rare 5-sigma events → fat-tailed.
        let mut bars = Vec::new();
        let mut c = 100.0;
        bars.push(mk_hp("2025-01-01", c, c + 0.1, c - 0.1, c));
        for i in 0..200 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let change = if i % 40 == 0 { 10.0 } else { 0.05 };
            c *= 1.0 + change / 100.0;
            bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
        }
        let snap = compute_retkurt_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.kurt_label != "INSUFFICIENT_DATA");
        assert!(snap.excess_kurtosis > 1.0, "expected fat-tailed, got {}", snap.excess_kurtosis);
    }

    #[test]
    fn compute_retkurt_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_retkurt_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.kurt_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_tailr_balanced() {
        // Symmetric small random moves → balanced tail ratio.
        let mut bars = Vec::new();
        let mut c = 100.0;
        bars.push(mk_hp("2025-01-01", c, c + 0.1, c - 0.1, c));
        for i in 0..200 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let change = if i % 2 == 0 { 1.0 } else { -1.0 };
            c *= 1.0 + change / 100.0;
            bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
        }
        let snap = compute_tailr_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.bias_label != "INSUFFICIENT_DATA");
        assert!(snap.tail_ratio > 0.5 && snap.tail_ratio < 2.0);
    }

    #[test]
    fn compute_tailr_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_tailr_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.bias_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_runlen_trending() {
        // Monotone up → one long run.
        let mut bars = Vec::new();
        for i in 0..100 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            let c = 100.0 + i as f64 * 0.5;
            bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
        }
        let snap = compute_runlen_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.trend_label != "INSUFFICIENT_DATA");
        assert!(snap.longest_up_run >= 30, "expected long up-run, got {}", snap.longest_up_run);
        assert_eq!(snap.longest_down_run, 0);
        assert!(snap.current_run_length > 0);
    }

    #[test]
    fn compute_runlen_choppy() {
        // Alternating up/down → average run length 1.
        let mut bars = Vec::new();
        let mut c = 100.0;
        bars.push(mk_hp("2025-01-01", c, c + 0.1, c - 0.1, c));
        for i in 0..100 {
            let date = format!("2025-{:02}-{:02}", (i / 20) + 1, (i % 20) + 1);
            c += if i % 2 == 0 { 1.0 } else { -1.0 };
            bars.push(mk_hp(&date, c, c + 0.1, c - 0.1, c));
        }
        let snap = compute_runlen_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.trend_label, "CHOPPY");
        assert!(snap.avg_up_run < 1.5);
        assert!(snap.avg_down_run < 1.5);
    }

    #[test]
    fn compute_runlen_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_runlen_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.trend_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn compute_dayrange_compressed() {
        // Wide ranges historically, tighter recently → compressed.
        let mut bars = Vec::new();
        for i in 0..253 {
            let date = format!("2025-{:02}-{:02}", (i / 21) + 1, (i % 21) + 1);
            let c = 100.0;
            let (h, l) = if i < 200 {
                (c + 2.0, c - 2.0) // wide historical
            } else {
                (c + 0.3, c - 0.3) // tight recent
            };
            bars.push(mk_hp(&date, c, h, l, c));
        }
        let snap = compute_dayrange_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.range_label == "TIGHT" || snap.range_label == "COMPRESSED",
            "expected compressed, got {}", snap.range_label);
        assert!(snap.compression_ratio < 1.0);
    }

    #[test]
    fn compute_dayrange_expanded() {
        // Tight historical, wide recent → expanded.
        let mut bars = Vec::new();
        for i in 0..253 {
            let date = format!("2025-{:02}-{:02}", (i / 21) + 1, (i % 21) + 1);
            let c = 100.0;
            let (h, l) = if i < 200 {
                (c + 0.3, c - 0.3)
            } else {
                (c + 3.0, c - 3.0)
            };
            bars.push(mk_hp(&date, c, h, l, c));
        }
        let snap = compute_dayrange_snapshot("AAA", "2026-04-15", &bars);
        assert!(snap.range_label == "EXPANDED" || snap.range_label == "VERY_EXPANDED",
            "expected expanded, got {}", snap.range_label);
        assert!(snap.compression_ratio > 1.0);
    }

    #[test]
    fn compute_dayrange_insufficient() {
        let bars = vec![mk_hp("2025-06-01", 100.0, 101.0, 99.0, 100.0)];
        let snap = compute_dayrange_snapshot("AAA", "2026-04-15", &bars);
        assert_eq!(snap.range_label, "INSUFFICIENT_DATA");
    }

    // ── ADR-130 Web article ingestion tests ──

    #[test]
    fn ingested_articles_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v23(&c).unwrap();
        let snap = IngestedArticlesSnapshot {
            symbol: "AAPL".into(),
            articles: vec![WebArticle {
                title: "iPhone sales beat".into(),
                url: "https://example.com/a".into(),
                source: "Reuters".into(),
                published_at: "2026-04-10".into(),
                summary: "Strong quarter.".into(),
                agent_used: "claude".into(),
                ingested_at: 1_700_000_000,
            }],
        };
        upsert_ingested_articles(&c, "AAPL", &snap).unwrap();
        let got = get_ingested_articles(&c, "AAPL").unwrap().unwrap();
        assert_eq!(got.articles.len(), 1);
        assert_eq!(got.articles[0].url, "https://example.com/a");
    }

    #[test]
    fn ingested_articles_append_dedupe_and_cap() {
        let c = Connection::open_in_memory().unwrap();
        create_research_tables_v23(&c).unwrap();
        let mk = |url: &str, ts: i64| WebArticle {
            url: url.into(), ingested_at: ts, ..Default::default()
        };
        let batch1 = vec![mk("u1", 100), mk("u2", 110)];
        let (added1, total1) = append_ingested_articles(&c, "AAA", batch1).unwrap();
        assert_eq!(added1, 2);
        assert_eq!(total1, 2);

        // Same URL newer timestamp should replace, not add.
        let batch2 = vec![mk("u1", 200), mk("u3", 210)];
        let (added2, total2) = append_ingested_articles(&c, "AAA", batch2).unwrap();
        assert_eq!(added2, 1);
        assert_eq!(total2, 3);

        // Cap at INGESTED_ARTICLES_MAX: inject 60 more unique URLs.
        let big: Vec<WebArticle> = (0..60).map(|i| mk(&format!("big{}", i), 300 + i)).collect();
        let (_, total3) = append_ingested_articles(&c, "AAA", big).unwrap();
        assert_eq!(total3, INGESTED_ARTICLES_MAX);

        let got = get_ingested_articles(&c, "AAA").unwrap().unwrap();
        // Most recent first: big59 should be at the top.
        assert_eq!(got.articles[0].url, "big59");
    }

    #[test]
    fn parse_ingest_block_extracts_articles() {
        let text = r#"
Some preamble from the agent.

===TYPHOON_INGEST===
[
  {"symbol": "AAPL", "title": "iPhone sales beat", "url": "https://r.com/a",
   "source": "Reuters", "published_at": "2026-04-10", "summary": "Strong.",
   "agent": "claude"},
  {"symbol": "aapl", "title": "Services growth", "url": "https://b.com/b",
   "source": "Bloomberg", "published": "2026-04-11", "summary": "Good.",
   "agent": "claude"},
  {"symbol": "MSFT", "title": "Azure outage", "url": "https://c.com/c",
   "source": "TheVerge", "date": "2026-04-09", "summary": "Brief.",
   "agent": "claude"}
]
===END_INGEST===

Trailing text.
"#;
        let parsed = parse_ingest_block(text);
        let by_sym: std::collections::HashMap<_, _> = parsed.into_iter().collect();
        assert_eq!(by_sym.get("AAPL").map(|v| v.len()), Some(2));
        assert_eq!(by_sym.get("MSFT").map(|v| v.len()), Some(1));
        let msft = &by_sym["MSFT"][0];
        assert_eq!(msft.published_at, "2026-04-09");
        assert_eq!(msft.agent_used, "claude");
    }

    #[test]
    fn parse_ingest_block_with_json_fence() {
        let text = r#"
===TYPHOON_INGEST===
```json
[
  {"symbol": "NVDA", "title": "Blackwell", "url": "https://x.com/n",
   "source": "CNBC", "published_at": "2026-04-12", "summary": "Demand.",
   "agent": "gemini"}
]
```
===END_INGEST===
"#;
        let parsed = parse_ingest_block(text);
        assert_eq!(parsed.len(), 1);
        assert_eq!(parsed[0].0, "NVDA");
        assert_eq!(parsed[0].1.len(), 1);
    }

    #[test]
    fn parse_ingest_block_skips_malformed_entries() {
        let text = r#"
===TYPHOON_INGEST===
[
  {"symbol": "AAPL"},
  {"url": "https://no-symbol.com/x"},
  {"symbol": "TSLA", "url": "https://good.com/g"}
]
===END_INGEST===
"#;
        let parsed = parse_ingest_block(text);
        let by_sym: std::collections::HashMap<_, _> = parsed.into_iter().collect();
        assert_eq!(by_sym.get("TSLA").map(|v| v.len()), Some(1));
        assert!(!by_sym.contains_key("AAPL"));
    }

    #[test]
    fn parse_ingest_block_returns_empty_when_missing() {
        let text = "No ingest block here.";
        let parsed = parse_ingest_block(text);
        assert!(parsed.is_empty());
    }

    // ── ADR-131 Round 23 tests ──

    fn synthetic_up_trend_bars() -> Vec<HistoricalPriceRow> {
        // 60 bars, each slightly higher than the previous (deterministic
        // drift). Simulates a persistent uptrend: Hurst should be > 0.5,
        // hit rate should be high, GLASYM ratio should be ≥ 1, AUTOCOR
        // lag 1 ~ 0.
        (0..60).map(|i| {
            let close = 100.0 + (i as f64) * 0.5;
            HistoricalPriceRow {
                date: format!("2025-{:02}-{:02}", 1 + (i / 20) as u32, 1 + (i % 20) as u32),
                open: close - 0.25,
                high: close + 0.5,
                low: close - 0.75,
                close,
                adj_close: close,
                volume: 1_000_000.0 + (i as f64) * 50_000.0,
                change: 0.5,
                change_pct: 0.5,
            }
        }).collect()
    }

    fn synthetic_mixed_bars() -> Vec<HistoricalPriceRow> {
        // 60 bars, alternating up/down ~equally — tests the BALANCED /
        // NEUTRAL / RANDOM_WALK paths.
        (0..60).map(|i| {
            let base = 100.0;
            let close = if i % 2 == 0 { base + 1.0 } else { base - 1.0 };
            HistoricalPriceRow {
                date: format!("2025-{:02}-{:02}", 1 + (i / 20) as u32, 1 + (i % 20) as u32),
                open: base,
                high: base + 1.5,
                low: base - 1.5,
                close,
                adj_close: close,
                volume: if i % 2 == 0 { 2_000_000.0 } else { 1_000_000.0 },
                change: 0.0,
                change_pct: 0.0,
            }
        }).collect()
    }

    #[test]
    fn autocor_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = AutocorrelationSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 200,
            lag1_acf: -0.12,
            lag5_acf: 0.02,
            lag10_acf: -0.01,
            lag20_acf: 0.03,
            mean_log_return: 0.0008,
            regime_label: "MEAN_REVERT".into(),
            note: String::new(),
        };
        upsert_autocor(&c, "TEST", &snap).unwrap();
        let got = get_autocor(&c, "TEST").unwrap().unwrap();
        assert_eq!(got.symbol, "TEST");
        assert!((got.lag1_acf - -0.12).abs() < 1e-9);
        assert_eq!(got.regime_label, "MEAN_REVERT");
    }

    #[test]
    fn autocor_compute_insufficient_data() {
        let snap = compute_autocor_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn autocor_compute_uptrend_labels() {
        let bars = synthetic_up_trend_bars();
        let snap = compute_autocor_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.regime_label, "INSUFFICIENT_DATA");
        assert!(snap.bars_used >= 30);
    }

    #[test]
    fn hurst_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = HurstSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            hurst_exponent: 0.58,
            scales_used: 4,
            min_scale: 8,
            max_scale: 64,
            memory_label: "PERSISTENT".into(),
            note: String::new(),
        };
        upsert_hurst(&c, "TEST", &snap).unwrap();
        let got = get_hurst(&c, "TEST").unwrap().unwrap();
        assert!((got.hurst_exponent - 0.58).abs() < 1e-9);
        assert_eq!(got.memory_label, "PERSISTENT");
    }

    #[test]
    fn hurst_compute_insufficient_data() {
        let snap = compute_hurst_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.memory_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn hurst_compute_picks_label() {
        let bars = synthetic_mixed_bars();
        let snap = compute_hurst_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.memory_label, "INSUFFICIENT_DATA");
        assert!(snap.scales_used >= 2);
    }

    #[test]
    fn hitrate_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = HitRateSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            hitrate_5d: 60.0,
            hitrate_20d: 55.0,
            hitrate_60d: 52.0,
            hitrate_252d: 51.0,
            up_days: 130,
            down_days: 120,
            flat_days: 3,
            hit_label: "WEAK_BULLISH".into(),
            note: String::new(),
        };
        upsert_hitrate(&c, "TEST", &snap).unwrap();
        let got = get_hitrate(&c, "TEST").unwrap().unwrap();
        assert_eq!(got.up_days, 130);
        assert_eq!(got.hit_label, "WEAK_BULLISH");
    }

    #[test]
    fn hitrate_compute_uptrend_is_bullish() {
        let bars = synthetic_up_trend_bars();
        let snap = compute_hitrate_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.hit_label, "INSUFFICIENT_DATA");
        assert!(snap.up_days > snap.down_days, "uptrend should have more up days");
    }

    #[test]
    fn glasym_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = GainLossAsymmetrySnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            avg_up_pct: 1.2,
            avg_down_pct: 1.1,
            median_up_pct: 0.9,
            median_down_pct: 0.8,
            magnitude_ratio: 1.09,
            up_days: 130,
            down_days: 120,
            asymmetry_label: "BALANCED".into(),
            note: String::new(),
        };
        upsert_glasym(&c, "TEST", &snap).unwrap();
        let got = get_glasym(&c, "TEST").unwrap().unwrap();
        assert!((got.magnitude_ratio - 1.09).abs() < 1e-9);
        assert_eq!(got.asymmetry_label, "BALANCED");
    }

    #[test]
    fn glasym_compute_insufficient_when_empty() {
        let snap = compute_glasym_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.asymmetry_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn glasym_compute_mixed_is_balanced() {
        let bars = synthetic_mixed_bars();
        let snap = compute_glasym_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.asymmetry_label, "INSUFFICIENT_DATA");
        assert!(snap.up_days > 0 && snap.down_days > 0);
    }

    #[test]
    fn volratio_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = VolumeRatioSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 253,
            avg_up_volume: 2_500_000.0,
            avg_down_volume: 2_000_000.0,
            median_up_volume: 2_400_000.0,
            median_down_volume: 1_900_000.0,
            up_down_volume_ratio: 1.25,
            max_up_volume: 8_000_000.0,
            max_down_volume: 5_500_000.0,
            up_days: 130,
            down_days: 120,
            flow_label: "SLIGHT_ACCUMULATION".into(),
            note: String::new(),
        };
        upsert_volratio(&c, "TEST", &snap).unwrap();
        let got = get_volratio(&c, "TEST").unwrap().unwrap();
        assert!((got.up_down_volume_ratio - 1.25).abs() < 1e-9);
        assert_eq!(got.flow_label, "SLIGHT_ACCUMULATION");
    }

    #[test]
    fn volratio_compute_no_volume_returns_insufficient() {
        let bars: Vec<HistoricalPriceRow> = (0..30).map(|i| HistoricalPriceRow {
            date: format!("2025-01-{:02}", i + 1),
            open: 100.0, high: 101.0, low: 99.0, close: 100.0 + i as f64,
            adj_close: 100.0, volume: 0.0, change: 0.0, change_pct: 0.0,
        }).collect();
        let snap = compute_volratio_snapshot("X", "2026-04-15", &bars);
        assert_eq!(snap.flow_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn volratio_compute_with_volume() {
        let bars = synthetic_mixed_bars();
        let snap = compute_volratio_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.flow_label, "INSUFFICIENT_DATA");
        assert!(snap.up_days > 0 && snap.down_days > 0);
    }

    // ── ADR-132 Round 24 tests ──

    fn synthetic_gappy_bars() -> Vec<HistoricalPriceRow> {
        // 40 bars with intentional overnight gaps (open ≠ prior close).
        (0..40).map(|i| {
            let prior_close = 100.0 + (i as f64);
            let gap = if i % 5 == 0 { 1.5 } else if i % 7 == 0 { -1.2 } else { 0.2 };
            let open = prior_close + gap;
            let close = open + 0.3;
            HistoricalPriceRow {
                date: format!("2025-{:02}-{:02}", 1 + (i / 20) as u32, 1 + (i % 20) as u32),
                open,
                high: open + 0.8,
                low: open - 0.6,
                close,
                adj_close: close,
                volume: 1_000_000.0,
                change: 0.3,
                change_pct: 0.3,
            }
        }).collect()
    }

    #[test]
    fn drawup_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = DrawupHistorySnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 200,
            max_drawup_pct: 22.5,
            max_drawup_trough_date: "2025-06-01".into(),
            max_drawup_peak_date: "2025-09-15".into(),
            longest_drawup_days: 45,
            rallies_5pct: 4,
            rallies_10pct: 2,
            current_drawup_pct: 3.2,
            rally_label: "STRONG".into(),
            note: String::new(),
        };
        upsert_drawup(&c, "TEST", &snap).unwrap();
        let got = get_drawup(&c, "TEST").unwrap().unwrap();
        assert!((got.max_drawup_pct - 22.5).abs() < 1e-9);
        assert_eq!(got.rally_label, "STRONG");
    }

    #[test]
    fn drawup_compute_up_trend_is_explosive() {
        let bars = synthetic_up_trend_bars();
        let snap = compute_drawup_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.rally_label, "INSUFFICIENT_DATA");
        assert!(snap.max_drawup_pct > 0.0);
    }

    #[test]
    fn drawup_compute_insufficient() {
        let snap = compute_drawup_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.rally_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn gapstats_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = GapStatsSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 252,
            gap_up_count: 30,
            gap_down_count: 25,
            avg_gap_pct: 0.08,
            avg_gap_up_pct: 1.5,
            avg_gap_down_pct: -1.3,
            largest_gap_up_pct: 6.2,
            largest_gap_down_pct: -4.8,
            gap_frequency_pct: 21.83,
            bias_label: "SLIGHT_UP".into(),
            note: String::new(),
        };
        upsert_gapstats(&c, "TEST", &snap).unwrap();
        let got = get_gapstats(&c, "TEST").unwrap().unwrap();
        assert_eq!(got.gap_up_count, 30);
        assert_eq!(got.bias_label, "SLIGHT_UP");
    }

    #[test]
    fn gapstats_compute_with_gaps() {
        let bars = synthetic_gappy_bars();
        let snap = compute_gapstats_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.bias_label, "INSUFFICIENT_DATA");
        assert!(snap.gap_up_count > 0 || snap.gap_down_count > 0);
    }

    #[test]
    fn gapstats_compute_insufficient() {
        let snap = compute_gapstats_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.bias_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn volcluster_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = VolClusterSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 252,
            sq_acf_lag1: 0.18,
            sq_acf_lag5: 0.09,
            sq_acf_lag20: 0.05,
            abs_acf_lag1: 0.22,
            abs_acf_lag5: 0.12,
            abs_acf_lag20: 0.07,
            cluster_label: "MODERATE".into(),
            note: String::new(),
        };
        upsert_volcluster(&c, "TEST", &snap).unwrap();
        let got = get_volcluster(&c, "TEST").unwrap().unwrap();
        assert!((got.abs_acf_lag1 - 0.22).abs() < 1e-9);
        assert_eq!(got.cluster_label, "MODERATE");
    }

    #[test]
    fn volcluster_compute_insufficient() {
        let snap = compute_volcluster_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.cluster_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn closeplc_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = ClosePlacementSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 200,
            avg_placement: 0.65,
            median_placement: 0.70,
            latest_placement: 0.82,
            pct_near_high: 35.0,
            pct_near_low: 12.0,
            placement_label: "BULL".into(),
            note: String::new(),
        };
        upsert_closeplc(&c, "TEST", &snap).unwrap();
        let got = get_closeplc(&c, "TEST").unwrap().unwrap();
        assert!((got.avg_placement - 0.65).abs() < 1e-9);
        assert_eq!(got.placement_label, "BULL");
    }

    #[test]
    fn closeplc_compute_with_bars() {
        let bars = synthetic_up_trend_bars();
        let snap = compute_closeplc_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.placement_label, "INSUFFICIENT_DATA");
        assert!(snap.avg_placement >= 0.0 && snap.avg_placement <= 1.0);
    }

    #[test]
    fn closeplc_compute_insufficient() {
        let snap = compute_closeplc_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.placement_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn mrhl_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = MeanReversionHalfLifeSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 252,
            beta: 0.25,
            alpha: 0.0001,
            half_life_days: 0.5,
            r_squared: 0.06,
            regime_label: "NEUTRAL".into(),
            note: String::new(),
        };
        upsert_mrhl(&c, "TEST", &snap).unwrap();
        let got = get_mrhl(&c, "TEST").unwrap().unwrap();
        assert!((got.beta - 0.25).abs() < 1e-9);
        assert_eq!(got.regime_label, "NEUTRAL");
    }

    #[test]
    fn mrhl_compute_insufficient() {
        let snap = compute_mrhl_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.regime_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn mrhl_compute_with_bars() {
        let bars = synthetic_mixed_bars();
        let snap = compute_mrhl_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.regime_label, "INSUFFICIENT_DATA");
        assert!(snap.beta.is_finite());
    }

    // ── ADR-133 Round 25 tests ──

    #[test]
    fn downvol_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = DownsideVolSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 252,
            mean_log_return: 0.0008,
            downside_dev: 0.012,
            downside_dev_ann: 0.19,
            upside_dev: 0.011,
            sortino_ratio: 0.067,
            sortino_ratio_ann: 1.06,
            downside_pct_of_total: 50.5,
            sortino_label: "GOOD".into(),
            note: String::new(),
        };
        upsert_downvol(&c, "TEST", &snap).unwrap();
        let got = get_downvol(&c, "TEST").unwrap().unwrap();
        assert!((got.sortino_ratio_ann - 1.06).abs() < 1e-9);
        assert_eq!(got.sortino_label, "GOOD");
    }

    #[test]
    fn downvol_compute_insufficient() {
        let snap = compute_downvol_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.sortino_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn downvol_compute_uptrend_is_good() {
        let bars = synthetic_up_trend_bars();
        let snap = compute_downvol_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.sortino_label, "INSUFFICIENT_DATA");
        assert!(snap.mean_log_return > 0.0);
    }

    #[test]
    fn sharpr_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = SharpeRatioSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 252,
            mean_log_return: 0.0008,
            stdev_log_return: 0.012,
            sharpe_ratio: 0.067,
            sharpe_ratio_ann: 1.06,
            mean_return_ann: 0.2016,
            stdev_return_ann: 0.19,
            sharpe_label: "GOOD".into(),
            note: String::new(),
        };
        upsert_sharpr(&c, "TEST", &snap).unwrap();
        let got = get_sharpr(&c, "TEST").unwrap().unwrap();
        assert!((got.sharpe_ratio_ann - 1.06).abs() < 1e-9);
        assert_eq!(got.sharpe_label, "GOOD");
    }

    #[test]
    fn sharpr_compute_insufficient() {
        let snap = compute_sharpr_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.sharpe_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn sharpr_compute_uptrend_is_positive() {
        let bars = synthetic_up_trend_bars();
        let snap = compute_sharpr_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.sharpe_label, "INSUFFICIENT_DATA");
        assert!(snap.sharpe_ratio > 0.0);
    }

    #[test]
    fn effratio_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = EfficiencyRatioSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 252,
            start_close: 100.0,
            end_close: 130.0,
            net_change: 30.0,
            net_change_pct: 30.0,
            sum_abs_changes: 50.0,
            efficiency_ratio: 0.6,
            signed_efficiency: 0.6,
            efficiency_label: "STRONG_TREND".into(),
            note: String::new(),
        };
        upsert_effratio(&c, "TEST", &snap).unwrap();
        let got = get_effratio(&c, "TEST").unwrap().unwrap();
        assert!((got.efficiency_ratio - 0.6).abs() < 1e-9);
        assert_eq!(got.efficiency_label, "STRONG_TREND");
    }

    #[test]
    fn effratio_compute_uptrend_is_trending() {
        let bars = synthetic_up_trend_bars();
        let snap = compute_effratio_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.efficiency_label, "INSUFFICIENT_DATA");
        assert!(snap.efficiency_ratio > 0.5, "strictly monotone bars should be highly efficient, got {}", snap.efficiency_ratio);
    }

    #[test]
    fn effratio_compute_chop_is_low() {
        let bars = synthetic_mixed_bars();
        let snap = compute_effratio_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.efficiency_label, "INSUFFICIENT_DATA");
        assert!(snap.efficiency_ratio < 0.5, "alternating bars should be choppy, got {}", snap.efficiency_ratio);
    }

    #[test]
    fn wickbias_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = WickBiasSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 252,
            avg_upper_wick: 0.18,
            avg_lower_wick: 0.22,
            median_upper_wick: 0.15,
            median_lower_wick: 0.2,
            avg_body_share: 0.6,
            wick_bias_score: 0.04,
            bias_label: "BUYER_LEAN".into(),
            note: String::new(),
        };
        upsert_wickbias(&c, "TEST", &snap).unwrap();
        let got = get_wickbias(&c, "TEST").unwrap().unwrap();
        assert!((got.wick_bias_score - 0.04).abs() < 1e-9);
        assert_eq!(got.bias_label, "BUYER_LEAN");
    }

    #[test]
    fn wickbias_compute_insufficient() {
        let snap = compute_wickbias_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.bias_label, "INSUFFICIENT_DATA");
    }

    #[test]
    fn wickbias_compute_with_bars() {
        let bars = synthetic_up_trend_bars();
        let snap = compute_wickbias_snapshot("X", "2026-04-15", &bars);
        assert_ne!(snap.bias_label, "INSUFFICIENT_DATA");
        let total = snap.avg_upper_wick + snap.avg_lower_wick + snap.avg_body_share;
        assert!((total - 1.0).abs() < 1e-6, "wick + body should sum to 1, got {}", total);
    }

    #[test]
    fn volofvol_snapshot_roundtrip() {
        let c = Connection::open_in_memory().unwrap();
        let snap = VolOfVolSnapshot {
            symbol: "TEST".into(),
            as_of: "2026-04-15".into(),
            bars_used: 233,
            mean_rv20: 0.012,
            stdev_rv20: 0.003,
            min_rv20: 0.008,
            max_rv20: 0.018,
            latest_rv20: 0.013,
            cv_rv20: 0.25,
            cv_label: "MODERATE".into(),
            note: String::new(),
        };
        upsert_volofvol(&c, "TEST", &snap).unwrap();
        let got = get_volofvol(&c, "TEST").unwrap().unwrap();
        assert!((got.cv_rv20 - 0.25).abs() < 1e-9);
        assert_eq!(got.cv_label, "MODERATE");
    }

    #[test]
    fn volofvol_compute_insufficient() {
        let snap = compute_volofvol_snapshot("X", "2026-04-15", &[]);
        assert_eq!(snap.cv_label, "INSUFFICIENT_DATA");
    }
}
