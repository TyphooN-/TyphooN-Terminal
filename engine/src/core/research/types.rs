use serde::{Deserialize, Serialize};

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
