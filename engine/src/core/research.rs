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
}
