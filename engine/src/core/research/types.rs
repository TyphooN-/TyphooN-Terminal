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
    pub description: String,     // long "About" text
    pub market_cap: f64,         // in USD millions (Finnhub native unit)
    pub shares_outstanding: f64, // in millions
}

/// One row in the earnings history (actual vs estimate EPS).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarningRow {
    pub period: String, // YYYY-MM-DD
    pub actual: Option<f64>,
    pub estimate: Option<f64>,
    pub surprise: Option<f64>,
    pub surprise_pct: Option<f64>,
    pub quarter: Option<i32>,
    pub year: Option<i32>,
}

/// Corporate action event (split, dividend, spin-off, etc.).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorporateAction {
    pub symbol: String,
    pub date: String,        // YYYY-MM-DD
    pub action_type: String, // "split", "dividend", "spin", etc.
    pub value: f64,          // split ratio or dividend amount
    pub currency: Option<String>,
    pub note: Option<String>,
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


/// Commodity futures quote row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CommodityQuote {
    pub symbol: String,  // e.g. "GC=F"
    pub display: String, // e.g. "Gold"
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
}


// ── Godel Parity Round 3 types ─────────────────────────────────────

/// FA — one fiscal period of an Income Statement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IncomeStatement {
    pub date: String,   // period end YYYY-MM-DD
    pub period: String, // "FY" | "Q1" | "Q2" | "Q3" | "Q4"
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
    pub since: String,     // year joined role (string to handle Finnhub "N/A")
    pub compensation: f64, // USD total comp for the year
    pub year: i32,         // comp reporting year
}

/// COT — one CFTC Commitment of Traders weekly row (legacy futures).
/// Global snapshot, not per-symbol. Not persisted (weekly refresh is fast, staleness meaningless).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CotReport {
    pub market_name: String, // e.g. "GOLD - COMMODITY EXCHANGE INC."
    pub market_code: String, // CFTC contract market code
    pub report_date: String, // YYYY-MM-DD
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

// ── Godel Parity Round 4 types ─────────────────────────────────────

/// SPLT — one historical stock split event.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StockSplit {
    pub date: String,     // YYYY-MM-DD
    pub label: String,    // "2:1" | "3:2" etc.
    pub numerator: f64,   // new shares
    pub denominator: f64, // old shares
}

/// ETF — one constituent holding of an exchange-traded fund.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EtfHolding {
    pub symbol: String,  // held company ticker
    pub name: String,    // held company name
    pub weight_pct: f64, // % of ETF AUM
    pub shares: f64,
    pub market_value: f64,
    pub updated: String, // as-of date
}

/// ANR — analyst recommendation bucket trend for a single period.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnalystRecommendation {
    pub period: String, // YYYY-MM-DD (end of reporting month)
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
    pub last_updated: String, // YYYY-MM-DD
    pub num_analysts: i32,
}

/// ESG — environmental / social / governance risk score.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EsgScore {
    pub symbol: String,
    pub environmental_score: f64,
    pub social_score: f64,
    pub governance_score: f64,
    pub esg_score: f64, // weighted composite
    pub year: i32,
}

/// MEMB — one member company of an equity index.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexMember {
    pub index: String, // "SP500" | "NDX" | "DJIA"
    pub symbol: String,
    pub name: String,
    pub sector: String,
    pub sub_sector: String,
    pub headquarters: String,
    pub date_added: String, // YYYY-MM-DD when admitted to index
}

// ── Godel Parity Round 5 ─────────────────────────────────────────

/// INS — one insider trade filing (Form 4 row).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsiderTrade {
    pub filing_date: String,             // YYYY-MM-DD when filed with SEC
    pub transaction_date: String,        // YYYY-MM-DD of the trade itself
    pub reporting_name: String,          // insider who filed
    pub transaction_type: String,        // "P-Purchase", "S-Sale", "M-Exempt", "A-Award", etc.
    pub acquisition_disposition: String, // "A" (acquired) or "D" (disposed)
    pub shares: f64,                     // securitiesTransacted
    pub price: f64,                      // per-share price
    pub value_usd: f64,                  // shares * price (derived)
    pub shares_owned_after: f64,         // securitiesOwned post-trade
    pub link: String,                    // SEC EDGAR filing URL
}

/// HDS — one institutional holder row (13F-derived).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InstitutionalHolder {
    pub holder: String,        // fund / manager name
    pub shares: f64,           // shares held
    pub date_reported: String, // 13F as-of date
    pub change: f64,           // delta shares vs prior quarter
}

/// FLOAT — shares float breakdown snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SharesFloat {
    pub symbol: String,
    pub date: String,            // YYYY-MM-DD snapshot date
    pub free_float_pct: f64,     // % of outstanding that is free-float
    pub float_shares: f64,       // absolute free float
    pub outstanding_shares: f64, // total shares outstanding
    pub source: String,          // data provider
}

/// HP — one OHLCV daily bar for historical price table.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HistoricalPriceRow {
    pub date: String, // YYYY-MM-DD
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub adj_close: f64,
    pub volume: f64,
    pub change: f64,     // close - open (USD)
    pub change_pct: f64, // % change (close vs prior close)
}

/// EPS — one earnings surprise row (actual vs estimate).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarningsSurprise {
    pub date: String, // report date YYYY-MM-DD
    pub symbol: String,
    pub eps_actual: f64,
    pub eps_estimate: f64,
    pub surprise: f64,     // actual - estimate
    pub surprise_pct: f64, // (actual - estimate) / |estimate| * 100
}

// ── Godel Parity Round 6 ─────────────────────────────────────────

/// WEI — one global equity index quote row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WorldIndex {
    pub ticker: String,  // Yahoo ticker e.g. "^GSPC"
    pub display: String, // human name "S&P 500"
    pub region: String,  // "Americas" | "Europe" | "Asia-Pacific"
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
    pub sector: String,  // "Technology", "Energy", …
    pub change_pct: f64, // % change (absolute, e.g. 1.23 = +1.23 %)
}

/// WACC — derived weighted-average cost of capital snapshot.
/// Built from FMP profile/key-metrics + cached GY 10Y yield (risk-free rate)
/// using the standard CAPM cost-of-equity and after-tax cost-of-debt formulas.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WaccSnapshot {
    pub symbol: String,
    pub as_of: String,                 // YYYY-MM-DD snapshot date (usually "today")
    pub beta: f64,                     // equity beta from FMP profile
    pub risk_free_pct: f64,            // 10Y Treasury yield %
    pub equity_risk_premium_pct: f64,  // assumed ERP (5.0 % default)
    pub cost_of_equity_pct: f64,       // Rf + β × ERP
    pub pre_tax_cost_of_debt_pct: f64, // interest expense / total debt × 100
    pub tax_rate_pct: f64,             // effective tax rate %
    pub after_tax_cost_of_debt_pct: f64, // pre-tax × (1 - tax_rate)
    pub market_cap: f64,               // equity market value (USD)
    pub total_debt: f64,               // book debt (USD, proxy for market debt)
    pub equity_weight: f64,            // E / (E+D)  (0..1)
    pub debt_weight: f64,              // D / (E+D)  (0..1)
    pub wacc_pct: f64,                 // we * Re + wd * Rd_after_tax
}

/// Hardcoded global equity index universe for the WEI dashboard.
/// Yahoo index tickers — all free via /v7/finance/quote.
pub const WORLD_INDICES_UNIVERSE: &[(&str, &str, &str)] = &[
    // Americas
    ("^GSPC", "S&P 500", "Americas"),
    ("^DJI", "Dow Jones", "Americas"),
    ("^IXIC", "Nasdaq Composite", "Americas"),
    ("^RUT", "Russell 2000", "Americas"),
    ("^GSPTSE", "S&P/TSX Composite", "Americas"),
    ("^BVSP", "Ibovespa", "Americas"),
    ("^MXX", "IPC Mexico", "Americas"),
    // Europe / Middle East / Africa
    ("^FTSE", "FTSE 100", "EMEA"),
    ("^GDAXI", "DAX", "EMEA"),
    ("^FCHI", "CAC 40", "EMEA"),
    ("^STOXX50E", "Euro Stoxx 50", "EMEA"),
    ("^IBEX", "IBEX 35", "EMEA"),
    ("FTSEMIB.MI", "FTSE MIB", "EMEA"),
    ("^AEX", "AEX", "EMEA"),
    ("^SSMI", "SMI", "EMEA"),
    // Asia-Pacific
    ("^N225", "Nikkei 225", "Asia-Pacific"),
    ("^HSI", "Hang Seng", "Asia-Pacific"),
    ("000001.SS", "Shanghai Composite", "Asia-Pacific"),
    ("^AXJO", "S&P/ASX 200", "Asia-Pacific"),
    ("^KS11", "KOSPI", "Asia-Pacific"),
    ("^TWII", "TSEC (Taiwan)", "Asia-Pacific"),
    ("^BSESN", "BSE SENSEX", "Asia-Pacific"),
];

/// Default equity risk premium used in the WACC CAPM calc (Damodaran-style).
pub const DEFAULT_EQUITY_RISK_PREMIUM_PCT: f64 = 5.0;

// ── Godel Parity Round 7 ─────────────────────────────────────────
// WCR / BETA / DDM / RV / FIGI surfaces.

/// WCR — single currency-cross row for the World Currency Rates dashboard.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrencyRate {
    pub ticker: String,  // Yahoo ticker, e.g. "EURUSD=X"
    pub display: String, // "EUR/USD"
    pub base: String,    // "EUR"
    pub quote: String,   // "USD"
    pub region: String,  // "Majors" / "Crosses" / "EM"
    pub price: f64,
    pub change: f64,
    pub change_pct: f64,
}

/// BETA — one rolling-window beta observation (e.g. 1Y/3Y/5Y).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BetaWindow {
    pub window_label: String, // "1Y" / "3Y" / "5Y"
    pub window_days: usize,   // trading-day window (252 / 756 / 1260)
    pub beta: f64,            // cov(r_s, r_m) / var(r_m)
    pub alpha_pct: f64,       // annualized intercept
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
    pub note: String, // any caveats (insufficient data, etc.)
}

/// DDM — Gordon Growth (two-stage optional) dividend discount model snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DdmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub annual_dividend: f64,     // trailing 4-quarter dividend $
    pub implied_growth_pct: f64,  // inferred from historical dividend CAGR
    pub required_return_pct: f64, // from WACC or cost of equity
    pub growth_source: String,    // "dividend CAGR 5Y" etc.
    pub return_source: String,    // "WACC 10.25%" etc.
    pub implied_price: f64,       // D1 / (r - g) — 0.0 when r <= g
    pub method: String,           // "Gordon Growth"
    pub note: String,             // any caveats
}

/// RV — one metric row in the relative-valuation peer matrix.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RvMetricRow {
    pub metric: String, // "P/E", "P/B", "EV/EBITDA", etc.
    pub value: f64,     // subject symbol's value
    pub peer_median: f64,
    pub peer_low: f64,
    pub peer_high: f64,
    pub z_score: f64,    // (value - mean) / stdev
    pub percentile: f64, // 0..100 within peer set
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
    pub figi: String, // share-class / instrument FIGI
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
    ("GC=F", "Gold", "Metals"),
    ("SI=F", "Silver", "Metals"),
    ("PL=F", "Platinum", "Metals"),
    ("PA=F", "Palladium", "Metals"),
    ("HG=F", "Copper", "Metals"),
    // Energy
    ("CL=F", "WTI Crude", "Energy"),
    ("BZ=F", "Brent Crude", "Energy"),
    ("NG=F", "Natural Gas", "Energy"),
    ("HO=F", "Heating Oil", "Energy"),
    ("RB=F", "Gasoline", "Energy"),
    // Grains
    ("ZC=F", "Corn", "Grains"),
    ("ZS=F", "Soybeans", "Grains"),
    ("ZW=F", "Wheat", "Grains"),
    ("ZO=F", "Oats", "Grains"),
    ("ZR=F", "Rice", "Grains"),
    // Softs
    ("KC=F", "Coffee", "Softs"),
    ("SB=F", "Sugar", "Softs"),
    ("CT=F", "Cotton", "Softs"),
    ("CC=F", "Cocoa", "Softs"),
    ("OJ=F", "Orange Juice", "Softs"),
    // Livestock
    ("LE=F", "Live Cattle", "Livestock"),
    ("HE=F", "Lean Hogs", "Livestock"),
    ("GF=F", "Feeder Cattle", "Livestock"),
];

// ── Godel Parity Round 8 ─────────────────────────────────────────
// HRA / DCF / SVM / OMON / IVOL surfaces.

/// HRA — one rolling-period return row (e.g. 1M, 3M, 1Y, YTD).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HraWindow {
    pub label: String, // "1D" / "5D" / "1M" / "3M" / "6M" / "YTD" / "1Y" / "3Y" / "5Y" / "ITD"
    pub trading_days: usize, // 0 for YTD/ITD which span by date
    pub return_pct: f64, // simple return (pct)
    pub cagr_pct: f64, // annualized when trading_days > 252
    pub n_observations: usize,
}

/// HRA — historical return + risk snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HraSnapshot {
    pub symbol: String,
    pub as_of: String, // YYYY-MM-DD
    pub last_close: f64,
    pub windows: Vec<HraWindow>,
    pub max_drawdown_pct: f64, // ITD, negative number
    pub drawdown_peak_date: String,
    pub drawdown_trough_date: String,
    pub volatility_annual_pct: f64, // stdev of daily log-returns × sqrt(252) × 100
    pub sharpe_ratio: f64,          // (mean daily return - rf) / stdev, annualized
    pub sortino_ratio: f64,         // same but downside deviation denominator
    pub calmar_ratio: f64,          // CAGR / |max_drawdown|
    pub risk_free_pct: f64,         // used in Sharpe/Sortino
    pub note: String,
}

/// DCF — one projection year in the explicit forecast period.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DcfYear {
    pub year: i32, // calendar year or offset
    pub revenue: f64,
    pub ebit: f64,
    pub nopat: f64, // NOPAT = EBIT × (1 - t)
    pub fcff: f64,  // free cash flow to firm
    pub discount_factor: f64,
    pub pv_fcff: f64, // fcff × discount_factor
}

/// DCF — Discounted Cash Flow fair value snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DcfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub method: String, // "DCF on FCFF"
    pub base_revenue: f64,
    pub base_fcff: f64,
    pub growth_pct: f64,          // explicit-period revenue growth
    pub terminal_growth_pct: f64, // Gordon growth in perpetuity
    pub wacc_pct: f64,            // discount rate
    pub tax_rate_pct: f64,
    pub fcff_margin_pct: f64, // fcff / revenue applied to projections
    pub projection_years: usize,
    pub years: Vec<DcfYear>,
    pub pv_sum: f64,           // Σ pv of explicit FCFF
    pub terminal_value: f64,   // TV at end of explicit period
    pub pv_terminal: f64,      // TV × final discount factor
    pub enterprise_value: f64, // pv_sum + pv_terminal
    pub total_debt: f64,
    pub cash_and_equivalents: f64,
    pub equity_value: f64, // EV - debt + cash
    pub shares_outstanding: f64,
    pub implied_price: f64, // equity_value / shares
    pub note: String,
}

/// SVM — one row in the multi-model fair-value triangulation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SvmModelRow {
    pub model: String, // "WACC cost of equity" / "DDM Gordon Growth" / "DCF FCFF" / "RV P/E median" / "RV EV/EBITDA median"
    pub implied_price: f64, // 0.0 if N/A
    pub current_price: f64,
    pub upside_pct: f64,    // (implied / current - 1) × 100
    pub confidence: String, // "high" / "medium" / "low" / "n/a"
    pub source: String,     // short lineage
}

/// SVM — Stock Valuation Model summary for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SvmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub current_price: f64,
    pub rows: Vec<SvmModelRow>,
    pub fair_low: f64,       // min of non-zero implied prices
    pub fair_high: f64,      // max of non-zero implied prices
    pub fair_mid: f64,       // simple mean of non-zero implied prices
    pub upside_mid_pct: f64, // (fair_mid / current - 1) × 100
    pub note: String,
}

/// OMON — one options contract row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptionContract {
    pub contract_symbol: String, // e.g. "AAPL240419C00150000"
    pub option_type: String,     // "CALL" / "PUT"
    pub strike: f64,
    pub last_price: f64,
    pub bid: f64,
    pub ask: f64,
    pub volume: f64,
    pub open_interest: f64,
    pub implied_volatility: f64, // decimal (0.25 = 25%)
    pub in_the_money: bool,
}

/// OMON — one expiration's call+put chain.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OptionExpiry {
    pub expiration: String, // YYYY-MM-DD
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
    pub date: String, // YYYY-MM-DD
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
    pub iv_rank: f64,       // 0..100: (current - low) / (high - low) × 100
    pub iv_percentile: f64, // 0..100: % of days at or below current
    pub observation_count: usize,
    pub history: Vec<IvolObservation>,
    pub note: String,
}

// ── Godel Parity Round 9 ─────────────────────────────────────────
// SEAG / COR / TRA / TECH / SKEW surfaces — all pure compute over existing
// HP / DVD / OMON caches, zero new API dependencies.

/// SEAG — one month's historical seasonality bucket (Jan..Dec).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeasonalityMonth {
    pub month: u32,          // 1..12
    pub label: String,       // "Jan", "Feb", …
    pub avg_return_pct: f64, // mean monthly return across years
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
    pub dow: u32,            // 1..7 (Mon=1, Sun=7)
    pub label: String,       // "Mon", "Tue", …
    pub avg_return_pct: f64, // mean daily log-return
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
    pub correlation: f64, // Pearson on daily log-returns
    pub n_observations: usize,
    pub beta_vs_peer: f64, // slope of ln(subject) vs ln(peer)
}

/// COR — Correlation matrix for a subject vs its peer set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationMatrix {
    pub symbol: String,
    pub as_of: String,
    pub window_days: usize, // e.g. 252 (1Y)
    pub cells: Vec<CorrelationCell>,
    pub mean_correlation: f64, // average |ρ| across cells
    pub highest_corr_symbol: String,
    pub lowest_corr_symbol: String,
    pub note: String,
}

/// TRA — one total-return window (price return + dividend yield).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TotalReturnWindow {
    pub label: String, // "1M" / "3M" / "6M" / "YTD" / "1Y" / "3Y" / "5Y"
    pub trading_days: usize,
    pub price_return_pct: f64,
    pub dividend_yield_pct: f64, // dividends paid in window / start price × 100
    pub total_return_pct: f64,   // price + dividend yield (simple, not compound)
    pub annualized_pct: f64,     // annualized for windows ≥ 1Y, else simple
    pub dividends_paid: f64,     // cash per share in window
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
    pub name: String, // "RSI(14)", "MACD(12,26,9)", "BB(20,2)", "ATR(14)", "ADX(14)", "Stoch(14,3)"
    pub value: f64,   // primary value (for MACD this is the histogram)
    pub value_secondary: f64, // signal line / middle band / +DI / etc.
    pub value_tertiary: f64, // -DI / lower band / …
    pub signal: String, // "overbought" / "oversold" / "bullish" / "bearish" / "neutral"
    pub note: String, // short contextual hint
}

/// TECH — Technical indicator snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TechnicalSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub last_close: f64,
    pub indicators: Vec<TechnicalIndicator>,
    pub trend_summary: String, // short synthesized label
    pub note: String,
}

/// SKEW — one strike row on a volatility smile curve.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkewPoint {
    pub strike: f64,
    pub moneyness_pct: f64, // (strike / underlying - 1) × 100
    pub call_iv_pct: f64,
    pub put_iv_pct: f64,
    pub combined_iv_pct: f64, // average of call/put when both present
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

// ── Godel Parity Round 10 ───────────────────────────────────────────

/// LEV — one leverage / coverage ratio row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeverageRatio {
    pub name: String,
    pub value: f64,
    pub peer_median: f64, // 0.0 when unknown
    pub signal: String,   // "HEALTHY" | "ELEVATED" | "STRETCHED" | "NEUTRAL"
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
    pub period: String, // "FY2024" or "Q3 2024"
    pub date: String,   // YYYY-MM-DD
    pub net_income: f64,
    pub free_cash_flow: f64,
    pub fcf_to_ni_ratio: f64,     // FCF / NI
    pub cash_conversion_pct: f64, // FCF / NI × 100
    pub accruals: f64,            // NI - FCF
    pub quality_label: String,    // "HIGH" | "MEDIUM" | "LOW" | "NEGATIVE_NI"
}

/// ACRL — earnings quality snapshot (accruals vs cash flow conversion).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccrualsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub ttm_net_income: f64,
    pub ttm_free_cash_flow: f64,
    pub ttm_cash_conversion_pct: f64,
    pub avg_cash_conversion_pct: f64, // across the tracked periods
    pub periods: Vec<AccrualPeriod>,
    pub trend_label: String, // "IMPROVING" | "STABLE" | "DETERIORATING" | "MIXED"
    pub note: String,
}

/// RVOL — one realized-volatility window observation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealizedVolWindow {
    pub label: String, // "20d" / "60d" / "120d" / "252d"
    pub trading_days: usize,
    pub realized_vol_pct: f64, // annualized
    pub percentile: f64,       // 0..=100 — cone rank vs the full history of this window
    pub n_observations: usize,
}

/// RVOL — realized volatility + IV/RV gap snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealizedVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub last_close: f64,
    pub current_atm_iv_pct: f64, // from cached IVOL, 0.0 when unknown
    pub iv_rv_gap_pct: f64,      // IV − RV(20d)
    pub iv_rv_ratio: f64,        // IV / RV(20d)
    pub windows: Vec<RealizedVolWindow>,
    pub regime_label: String, // "CHEAP_IV" | "FAIR_IV" | "RICH_IV" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// FCFY — one dividend coverage / FCF yield row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FcfYieldPeriod {
    pub period: String,
    pub date: String,
    pub free_cash_flow: f64,
    pub dividends_paid: f64,
    pub payout_from_fcf_pct: f64, // dividends_paid / FCF × 100 (absolute cash-out ratio)
    pub payout_from_ni_pct: f64,  // dividends_paid / NI × 100
    pub fcf_yield_pct: f64, // FCF / market_cap_at_period × 100 (only TTM-level rows populate this)
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
    pub fcf_cagr_5y_pct: f64, // 0.0 when <5 years of annuals
    pub periods: Vec<FcfYieldPeriod>,
    pub sustainability_label: String, // "SAFE" | "STRETCHED" | "UNSUSTAINABLE" | "NO_DIVIDEND"
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
    pub days_to_cover: f64,         // short_shares / avg_daily_volume_20d
    pub short_ratio_reported: f64,  // from Fundamentals (vendor-provided, may differ)
    pub utilization_proxy_pct: f64, // short / float × 100 (same as short_percent_of_float but normalized)
    pub squeeze_risk_label: String, // "LOW" | "ELEVATED" | "HIGH" | "EXTREME" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// One short-interest history observation for a symbol.
/// Stored as a compact per-symbol time series and fed by fundamentals scrapes
/// plus explicit short-interest fetches when available.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ShortInterestHistoryPoint {
    pub as_of: String, // YYYY-MM-DD
    pub short_percent_of_float: f64,
    pub short_ratio: f64,
    pub shares_outstanding: f64,
}

// ── Godel Parity Round 11 ───────────────────────────────────────────

/// ALTZ — one component of the Altman Z-score.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AltmanComponent {
    pub name: String,      // e.g. "A: WC/TA"
    pub ratio: f64,        // raw ratio value
    pub coefficient: f64,  // 1.2 / 1.4 / 3.3 / 0.6 / 1.0
    pub contribution: f64, // coefficient × ratio
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
    pub z_score: f64, // sum of all contributions
    pub zone: String, // "DISTRESS" (<1.81) | "GRAY" | "SAFE" (>=2.99) | "INSUFFICIENT_DATA"
    pub components: Vec<AltmanComponent>,
    pub note: String,
}

/// PTFS — one Piotroski F-score check with signal.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PiotroskiCheck {
    pub category: String, // "Profitability" | "Leverage/Liquidity" | "Operating Efficiency"
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
    pub f_score: i32,             // 0..9
    pub strength_label: String,   // "STRONG" (>=7) | "MIXED" | "WEAK" (<=3) | "INSUFFICIENT_DATA"
    pub profitability_score: i32, // 0..4
    pub leverage_score: i32,      // 0..3
    pub efficiency_score: i32,    // 0..2
    pub checks: Vec<PiotroskiCheck>,
    pub note: String,
}

/// VOLE — one volatility estimator row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolEstimator {
    pub name: String, // "ClosedToClose" / "Parkinson" / "GarmanKlass" / "RogersSatchell" / "YangZhang"
    pub annualized_vol_pct: f64,
    pub efficiency_vs_close: f64, // multiplicative gain vs close-to-close (1.0 = same)
    pub note: String,
}

/// VOLE — OHLC volatility estimator snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OhlcVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub trading_days: usize,
    pub estimators: Vec<VolEstimator>,
    pub preferred_estimate_pct: f64, // Yang-Zhang when all 4 available, else Parkinson, else CtC
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
    pub beat_rate_pct: f64,  // beats / total × 100
    pub current_streak: i32, // positive = beat streak, negative = miss streak
    pub longest_beat_streak: usize,
    pub longest_miss_streak: usize,
    pub avg_surprise_pct: f64,
    pub median_surprise_pct: f64,
    pub recent_avg_surprise_pct: f64, // last 4 reports
    pub bias_label: String,           // "POSITIVE" | "NEGATIVE" | "NEUTRAL"
    pub trend_label: String,          // "ACCELERATING" | "STABLE" | "DECELERATING"
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
    pub dispersion_pct: f64, // (high - low) / mean × 100
    pub spread_pct: f64,     // (high - low) / current × 100
    pub implied_return_median_pct: f64,
    pub implied_return_mean_pct: f64,
    pub upside_to_high_pct: f64,
    pub downside_to_low_pct: f64,
    pub consensus_label: String, // "BULLISH" | "NEUTRAL" | "BEARISH" | "NO_COVERAGE"
    pub note: String,
}

// ── Godel Parity Round 12 ───────────────────────────────────────────

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
    pub other_count: usize, // awards, exercises, etc.
    pub unique_insiders: usize,
    pub gross_buy_value_usd: f64,
    pub gross_sell_value_usd: f64,
    pub net_value_usd: f64,  // buy - sell
    pub buy_sell_ratio: f64, // buy_count / max(sell_count, 1)
    pub net_shares: f64,     // buy_shares - sell_shares
    pub latest_trade_date: String,
    pub bias_label: String, // "BULLISH" | "NEUTRAL" | "BEARISH" | "NO_ACTIVITY"
    pub conviction_label: String, // "HIGH" | "MEDIUM" | "LOW" | "NONE"
    pub note: String,
}

/// DIVG — one annual-bucket dividend aggregation row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DivgAnnualRow {
    pub year: i32,
    pub total_amount: f64, // sum of cash dividends in the calendar year
    pub payment_count: usize,
    pub growth_pct: f64, // yoy % change vs prior year (0 if prior = 0)
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
    pub annualized_dividend: f64, // sum of most recent 4 payments
    pub years_covered: usize,
    pub cagr_1y_pct: f64, // year-over-year growth (latest annual bucket)
    pub cagr_3y_pct: f64, // 3-year CAGR
    pub cagr_5y_pct: f64, // 5-year CAGR
    pub consecutive_growth_years: usize,
    pub consistency_score_pct: f64, // % of yoy deltas that are non-negative
    pub annual_rows: Vec<DivgAnnualRow>,
    pub trend_label: String, // "GROWING" | "STABLE" | "CUTTING" | "NO_HISTORY"
    pub note: String,
}

/// EARM — one quarterly momentum row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarmQuarterRow {
    pub period: String, // "YYYY-MM-DD"
    pub revenue: f64,
    pub revenue_yoy_pct: f64, // vs year-ago quarter (same position + 4)
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
    pub composite_score: f64,   // 0..100 blended momentum score
    pub momentum_label: String, // "ACCELERATING" | "STABLE" | "DECELERATING" | "INSUFFICIENT_DATA"
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
    pub sector_rank: i32, // 1 = strongest, N = weakest
    pub sectors_total: i32,
    pub avg_sector_change_pct: f64,
    pub median_sector_change_pct: f64,
    pub relative_strength_pct: f64, // sector - avg
    pub breadth_pct: f64,           // % of sectors with positive change
    pub strongest_sector: String,
    pub strongest_sector_pct: f64,
    pub weakest_sector: String,
    pub weakest_sector_pct: f64,
    pub strength_label: String, // "LEADER" | "NEUTRAL" | "LAGGARD" | "NO_DATA"
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
    pub net_30d: i32, // upgrades - downgrades, 30d window
    pub net_90d: i32,
    pub net_180d: i32,
    pub latest_date: String,
    pub latest_action: String, // "upgrade" / "downgrade" / "initiation" / "maintain"
    pub latest_firm: String,
    pub latest_to_grade: String,
    pub bias_label: String, // "BULLISH" | "NEUTRAL" | "BEARISH" | "NO_COVERAGE"
    pub trend_label: String, // "IMPROVING" | "STABLE" | "DETERIORATING"
    pub note: String,
}

// ── Godel Parity Round 13 ───────────────────────────────────────────

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
    pub return_12_1_pct: f64,    // 12-month minus 1-month
    pub vol_annualized_pct: f64, // daily stdev × √252
    pub vol_adjusted_score: f64, // return_12_1 / vol_annualized
    pub composite_score: f64,    // 0..100 composite
    pub regime_label: String,    // "STRONG" | "NEUTRAL" | "WEAK" | "CRASH" | "INSUFFICIENT_DATA"
    pub trend_label: String,     // "ACCELERATING" | "STABLE" | "DECELERATING"
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
    pub daily_turnover_pct: f64, // avg share volume / shares out × 100
    pub amihud_illiquidity: f64, // 1e6 × mean(|return| / dollar volume)
    pub avg_true_range_pct: f64, // mean((high-low)/close) × 100
    pub spread_proxy_pct: f64,   // Corwin-Schultz high-low estimator
    pub liquidity_tier: String, // "DEEP" | "LIQUID" | "MODERATE" | "THIN" | "ILLIQUID" | "INSUFFICIENT_DATA"
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
    pub dist_from_52w_high_pct: f64, // (current - high) / high × 100 (negative when below)
    pub dist_from_52w_low_pct: f64,
    pub dist_from_20d_high_pct: f64,
    pub dist_from_60d_high_pct: f64,
    pub position_in_52w_range_pct: f64, // (current - low) / (high - low) × 100
    pub position_in_20d_range_pct: f64,
    pub consolidation_pct: f64, // 20d range / mean × 100
    pub breakout_label: String, // "NEW_HIGH" | "NEAR_HIGH" | "MID_RANGE" | "NEAR_LOW" | "NEW_LOW"
    pub setup_label: String, // "BREAKOUT_IMMINENT" | "CONSOLIDATING" | "TRENDING_UP" | "TRENDING_DOWN" | "NEUTRAL"
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
    pub efficiency_label: String, // "EFFICIENT" | "NEUTRAL" | "INEFFICIENT" | "INSUFFICIENT_DATA"
    pub trend_label: String,      // "IMPROVING" | "STABLE" | "DETERIORATING"
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
    pub composite_score: f64, // 0..100
    pub letter_grade: String, // "AAA" | "AA" | "A" | "BBB" | "BB" | "B" | "CCC" | "INSUFFICIENT_DATA"
    pub credit_label: String, // "INVESTMENT_GRADE" | "BORDERLINE" | "SPECULATIVE" | "DISTRESSED"
    pub inputs_available: usize,
    pub components: Vec<CreditComponent>,
    pub note: String,
}

// ── Godel Parity Round 14 ───────────────────────────────────────────

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
    pub momentum_score: f64, // from MOM composite
    pub momentum_regime: String,
    pub earnings_momentum_score: f64, // from EARM composite
    pub earnings_label: String,
    pub dividend_cagr_3y_pct: f64, // from DIVG
    pub dividend_trend: String,
    pub composite_score: f64, // 0..100
    pub garp_label: String,   // "GARP" | "GROWTH" | "VALUE" | "SPECULATIVE" | "NO_DATA"
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
    pub institutional_share_delta: f64, // sum of positive+negative HDS changes
    pub institutional_buyers: usize,    // count of holders with change > 0
    pub institutional_sellers: usize,   // count of holders with change < 0
    pub institutional_holders_tracked: usize,
    pub institutional_net_ratio: f64, // (buyers - sellers) / tracked
    pub insider_score: f64,           // 0..100
    pub institutional_score: f64,     // 0..100
    pub composite_score: f64,         // 0..100 weighted average
    pub flow_label: String, // "STRONG_BUY" | "BUY" | "NEUTRAL" | "SELL" | "STRONG_SELL" | "NO_DATA"
    pub note: String,
}

/// REGIME — Market regime classifier fusing VOLE + TECH + HRA snapshots.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RegimeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub realized_vol_pct: f64,     // from VOLE preferred_estimate_pct
    pub vol_source: String,        // "yang_zhang" | "parkinson" | "close_to_close"
    pub adx_value: f64,            // from TECH (ADX indicator)
    pub trend_summary: String,     // from TECH
    pub sharpe_ratio: f64,         // from HRA
    pub return_1y_pct: f64,        // from HRA
    pub trend_strength_score: f64, // 0..100 from ADX
    pub volatility_score: f64,     // 0..100 where lower vol = higher score
    pub return_score: f64,         // 0..100 from 1Y return
    pub composite_score: f64,      // 0..100
    pub regime_label: String, // "TRENDING" | "MEAN_REVERTING" | "VOLATILE" | "QUIET" | "INSUFFICIENT_DATA"
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
    pub rel_volume_5d: f64,         // current / 5d avg
    pub rel_volume_20d: f64,        // current / 20d avg
    pub rel_volume_60d: f64,        // current / 60d avg
    pub volume_trend_5d_pct: f64,   // (5d avg / 20d avg - 1) × 100
    pub volume_percentile_60d: f64, // rank of current_volume in the 60d sample, 0..=100
    pub activity_label: String, // "EXTREME" | "HIGH" | "ELEVATED" | "NORMAL" | "LOW" | "INSUFFICIENT_DATA"
    pub direction_label: String, // "BULLISH" | "BEARISH" | "NEUTRAL" (from current close vs prior)
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
    pub basis: String, // "annual" | "quarterly"
    pub latest_period: String,
    pub latest_gross_margin_pct: f64,
    pub latest_operating_margin_pct: f64,
    pub latest_net_margin_pct: f64,
    pub prior_gross_margin_pct: f64,
    pub prior_operating_margin_pct: f64,
    pub prior_net_margin_pct: f64,
    pub gross_margin_change_pct: f64, // latest - prior, in percentage points
    pub operating_margin_change_pct: f64,
    pub net_margin_change_pct: f64,
    pub avg_gross_margin_pct: f64, // across tracked periods
    pub avg_operating_margin_pct: f64,
    pub avg_net_margin_pct: f64,
    pub periods_used: usize,
    pub gross_trend_label: String, // "EXPANDING" | "STABLE" | "CONTRACTING"
    pub operating_trend_label: String,
    pub net_trend_label: String,
    pub overall_trend_label: String, // majority across the three
    pub quality_label: String,       // "HIGH" | "MEDIUM" | "LOW" (latest op margin bucket)
    pub periods: Vec<MarginRow>,
    pub note: String,
}

// ── Godel Parity Round 15 ───────────────────────────────────────────

/// Generic meta-composite sub-component row used by VAL / QUAL / RISK.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FactorComponent {
    pub name: String,
    pub value: String,
    pub score: f64, // 0..100 (higher = better for VAL/QUAL, higher = riskier for RISK)
    pub weight: f64, // raw percent weight
    pub contribution: f64,
}

/// VAL — Unified value-factor composite fusing valuation ratios vs sector peers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValueSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String, // sector used for peer medians
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
    pub value_label: String, // "DEEP_VALUE" | "VALUE" | "FAIR" | "EXPENSIVE" | "PREMIUM" | "NO_DATA"
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
    pub composite_score: f64,  // 0..100
    pub quality_label: String, // "HIGH_QUALITY" | "QUALITY" | "AVERAGE" | "POOR" | "WEAK" | "NO_DATA"
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
    pub composite_score: f64, // 0..100 — higher = RISKIER
    pub risk_label: String, // "LOW_RISK" | "MODERATE" | "ELEVATED" | "HIGH_RISK" | "DISTRESSED" | "NO_DATA"
    pub inputs_available: usize,
    pub components: Vec<FactorComponent>,
    pub note: String,
}

/// INSSTRK — One per-insider streak row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsiderStreakRow {
    pub insider_name: String,
    pub streak_direction: String, // "BUY" | "SELL" | "MIXED"
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
    pub buy_streak_count: usize, // insiders with ≥ 2 consecutive buys
    pub sell_streak_count: usize,
    pub longest_buy_streak: usize,
    pub longest_sell_streak: usize,
    pub net_buy_value_usd: f64,
    pub net_sell_value_usd: f64,
    pub streak_label: String, // "STRONG_ACCUMULATION" | "ACCUMULATION" | "DISTRIBUTION" | "STRONG_DISTRIBUTION" | "MIXED" | "NONE"
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
    pub consensus_bull_ratio: f64, // (strong_buy + buy) / total
    pub upgrades_90d: usize,
    pub downgrades_90d: usize,
    pub net_90d: i32,
    pub churn_90d: usize,       // upgrades + downgrades (total activity)
    pub breadth_score: f64,     // 0..100 (coverage size)
    pub consensus_score: f64,   // 0..100 (bullishness)
    pub churn_score: f64,       // 0..100 (activity)
    pub composite_score: f64,   // 0..100 weighted average
    pub coverage_label: String, // "EXPANDING" | "STABLE" | "CONTRACTING" | "THIN" | "NONE"
    pub inputs_available: usize,
    pub note: String,
}

// ── Godel Parity Round 16 ───────────────────────────────────────────

/// VRK — Value Rank vs sector peers snapshot.
/// Percentile rank of `ValueSnapshot.composite_score` within the same sector.
/// Higher percentile = better value (label ladder matches VAL's "DEEP_VALUE is good").
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValueRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub composite_score: f64,    // subject's VAL composite (copied)
    pub peers_considered: usize, // peers in the same sector with a VAL snapshot
    pub peers_with_data: usize,  // same as peers_considered today
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64, // 0..100 (higher = better value)
    pub rank_position: usize, // 1-based (1 = best value in cohort)
    pub rank_label: String, // "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" | "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" | "NO_DATA"
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
    pub percentile_rank: f64, // 0..100 (higher = better quality)
    pub rank_position: usize,
    pub rank_label: String, // same ladder as VRK
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
    pub composite_score: f64, // subject's RISK composite (higher = riskier)
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64, // 0..100 (higher = SAFER vs peers)
    pub rank_position: usize, // 1-based (1 = safest in cohort)
    pub rank_label: String, // "SAFEST_DECILE" | "SAFEST_QUARTILE" | "ABOVE_MEDIAN_SAFE" | "BELOW_MEDIAN_RISKY" | "BOTTOM_QUARTILE_RISKY" | "RISKIEST_DECILE" | "NO_DATA"
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
    pub gap_to_median_pp: f64, // symbol_cagr - sector_median (in percentage points)
    pub relative_label: String, // "FAR_ABOVE" | "ABOVE" | "INLINE" | "BELOW" | "FAR_BELOW" | "CAGR_NEGATIVE" | "NO_DATA"
    pub note: String,
}

/// PEAD — Per-event drift row (one per earnings announcement within the window).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PeadEventRow {
    pub event_date: String,
    pub surprise_pct: f64,
    pub classification: String, // "BEAT" | "MISS" | "INLINE"
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
    pub num_events: usize,  // surprises in the cache
    pub events_used: usize, // surprises successfully matched to HP bars
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

// ── Round 17 — size/momentum/drift rank + ops quality + revenue growth ──

/// SIZEF — Size Factor Rank snapshot.
/// Percentile rank of `Fundamentals.market_cap` within the same sector,
/// plus a tier label derived from absolute market cap.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SizeFactorSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub market_cap: f64,     // subject's market cap (USD)
    pub log_market_cap: f64, // ln(market_cap); 0 if cap <= 0
    pub tier_label: String, // "MEGA_CAP" | "LARGE_CAP" | "MID_CAP" | "SMALL_CAP" | "MICRO_CAP" | "NO_DATA"
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_cap: f64,
    pub sector_p25_cap: f64,
    pub sector_p75_cap: f64,
    pub percentile_rank: f64, // 0..100 (higher = larger within sector)
    pub rank_position: usize, // 1-based (1 = largest)
    pub rank_label: String,   // decile ladder — "TOP_DECILE" .. "BOTTOM_DECILE" | "NO_DATA"
    pub note: String,
}

/// MOMF — Momentum Factor Rank snapshot.
/// Percentile rank of `MomentumSnapshot.composite_score` within the same sector.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MomentumRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub composite_score: f64, // subject's MOM composite (copied)
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64, // 0..100 (higher = stronger momentum)
    pub rank_position: usize, // 1-based (1 = strongest)
    pub rank_label: String,   // same decile ladder as VRK/QRK
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
    pub avg_drift_5d_pct: f64, // subject's avg 5d drift (copied)
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_drift_5d_pct: f64,
    pub sector_p25_drift_5d_pct: f64,
    pub sector_p75_drift_5d_pct: f64,
    pub percentile_rank: f64, // 0..100 (higher = stronger positive drift)
    pub rank_position: usize, // 1-based (1 = strongest drift-up)
    pub rank_label: String,   // same decile ladder as VRK
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
    pub piotroski_score: i32, // 0..9
    pub piotroski_label: String,
    pub operating_margin_pct: f64,
    pub margin_trend_label: String, // EXPANDING / STABLE / CONTRACTING / MIXED
    pub cash_conversion_pct: f64,   // TTM cash conversion
    pub accruals_trend_label: String, // HIGH / STABLE / LOW / DETERIORATING
    pub composite_score: f64,       // 0..100
    pub operator_label: String, // "ELITE_OPERATOR" | "STRONG_OPERATOR" | "AVERAGE_OPERATOR" | "WEAK_OPERATOR" | "BROKEN_OPERATOR" | "NO_DATA"
    pub inputs_available: i32,  // 0..3 (PTFS/MARGINS/ACRL)
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
    pub gap_to_median_pp: f64,  // symbol_cagr - sector_median
    pub relative_label: String, // "FAR_ABOVE" | "ABOVE" | "INLINE" | "BELOW" | "FAR_BELOW" | "CAGR_NEGATIVE" | "NO_DATA"
    pub note: String,
}

// ── Round 18 — rank overlays + surprise streak ────────────────────

/// LEVRANK — Leverage Rank vs Sector Peers.
/// Percentile rank of debt-to-equity (`total_debt / total_equity`) from the
/// cached `LeverageSnapshot`, within the same sector. Inverted — lower D/E
/// = safer = higher rank. Uses RRK-style SAFEST label ladder.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeverageRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub debt_to_equity: f64, // subject's D/E (0 when equity non-positive)
    pub total_debt: f64,
    pub total_equity: f64,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_d2e: f64,
    pub sector_p25_d2e: f64,
    pub sector_p75_d2e: f64,
    pub percentile_rank: f64, // 0..100 (higher = SAFER, lower D/E)
    pub rank_position: usize, // 1-based (1 = safest)
    pub rank_label: String, // "SAFEST_DECILE" / ... / "RISKIEST_DECILE" / "NEGATIVE_EQUITY" / "NO_DATA"
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
    pub operating_margin_pct: f64,  // subject's latest op margin
    pub margin_trend_label: String, // copied from MarginsSnapshot.overall_trend_label
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_margin_pct: f64,
    pub sector_p25_margin_pct: f64,
    pub sector_p75_margin_pct: f64,
    pub percentile_rank: f64, // 0..100 (higher = fatter margins)
    pub rank_position: usize, // 1-based (1 = fattest)
    pub rank_label: String,   // standard decile ladder
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
    pub composite_score: f64,   // subject's FQM composite (copied)
    pub operator_label: String, // subject's FQM operator label (copied)
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64, // 0..100 (higher = better operator)
    pub rank_position: usize, // 1-based (1 = best operator)
    pub rank_label: String,   // standard decile ladder
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
    pub percentile_rank: f64, // 0..100 (higher = deeper liquidity)
    pub rank_position: usize, // 1-based (1 = deepest)
    pub rank_label: String,   // standard decile ladder
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
    pub total_events: usize, // events considered (BEAT/MISS/INLINE classification)
    pub beats: usize,
    pub misses: usize,
    pub inlines: usize,
    pub beat_rate_pct: f64,          // beats / total_events × 100
    pub current_streak_type: String, // "BEAT" | "MISS" | "INLINE" | "NONE"
    pub current_streak_len: usize,   // consecutive length of current streak
    pub longest_beat_streak: usize,
    pub longest_miss_streak: usize,
    pub avg_surprise_pct: f64,
    pub latest_event_date: String,
    pub latest_event_surprise_pct: f64,
    pub latest_event_label: String, // "BEAT" | "MISS" | "INLINE"
    pub streak_label: String, // "HOT_STREAK" | "BEAT_TREND" | "MIXED" | "MISS_TREND" | "COLD_STREAK" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 19 — dividend/earnings/rating rank overlays + gap/streak ─

/// DVDRANK — Dividend Growth Rank vs Sector Peers.
/// Percentile rank of `DivgSnapshot.cagr_3y_pct` within the same sector.
/// Higher CAGR = higher rank.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DividendGrowthRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub cagr_3y_pct: f64, // subject's 3y dividend CAGR (copied from DIVG)
    pub consecutive_growth_years: usize,
    pub trend_label: String, // subject's DIVG trend (copied, e.g. "GROWING")
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_cagr_pct: f64,
    pub sector_p25_cagr_pct: f64,
    pub sector_p75_cagr_pct: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String, // standard decile ladder
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
    pub momentum_label: String, // subject's EARM label (copied)
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
    pub net_90d: i32,       // subject's UPDM net_90d (copied)
    pub bias_label: String, // subject's UPDM bias (copied)
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
    pub bars_used: usize,    // sessions actually scanned (<=252)
    pub gaps_total: usize,   // non-zero gaps seen
    pub gaps_up_2pct: usize, // |gap| >= 2% and positive
    pub gaps_down_2pct: usize,
    pub gaps_up_5pct: usize,
    pub gaps_down_5pct: usize,
    pub gaps_up_10pct: usize,
    pub gaps_down_10pct: usize,
    pub largest_up_gap_pct: f64, // biggest positive gap seen (signed)
    pub largest_up_gap_date: String,
    pub largest_down_gap_pct: f64, // biggest negative gap seen (signed, negative)
    pub largest_down_gap_date: String,
    pub avg_abs_gap_pct: f64, // mean |gap| across all non-zero gaps
    pub gap_label: String,    // "EXPLOSIVE" | "GAPPY" | "NORMAL" | "SMOOTH" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// DES — Daily Event Streak snapshot. Pure time-series stat over the cached
/// HP daily bars. Tracks the current up/down close-over-close streak, the
/// longest up and down streaks in the window, plus a directional bias label.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyEventStreakSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,            // sessions actually scanned (<=252)
    pub current_streak_type: String, // "UP" | "DOWN" | "FLAT" | "NONE"
    pub current_streak_len: usize,
    pub longest_up_streak: usize,
    pub longest_down_streak: usize,
    pub up_days: usize,
    pub down_days: usize,
    pub flat_days: usize,
    pub up_day_rate_pct: f64, // up_days / (up+down) × 100
    pub avg_up_move_pct: f64, // mean % change on up days
    pub avg_down_move_pct: f64,
    pub streak_label: String, // "STRONG_UPTREND" | "UPTREND_BIAS" | "NEUTRAL" | "DOWNTREND_BIAS" | "STRONG_DOWNTREND" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 20 — yield/short rank + HP volatility/drawdown/returns ──

/// DVDYIELDRANK — Dividend Yield Rank vs Sector Peers.
/// Percentile rank of `Fundamentals.dividend_yield` within the same sector.
/// Non-payers (`dividend_yield.is_none() || dividend_yield == 0.0`) are
/// filtered out so the cohort captures dividend-paying names only.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DividendYieldRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub dividend_yield_pct: f64, // subject's current dividend yield %
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_yield_pct: f64,
    pub sector_p25_yield_pct: f64,
    pub sector_p75_yield_pct: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String, // standard decile ladder
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
    pub rank_label: String, // risk-inverted: SAFEST_DECILE (lowest short) → RISKIEST_DECILE
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
    pub bars_used: usize, // sessions in the window (<=253)
    pub latest_close: f64,
    pub atr14: f64,              // 14-period Wilder ATR in price units
    pub atr14_pct: f64,          // atr14 / latest_close × 100
    pub atr_annualized_pct: f64, // atr14_pct × √252
    pub regime_label: String, // "LOW_VOL" | "NORMAL_VOL" | "HIGH_VOL" | "EXTREME_VOL" | "INSUFFICIENT_DATA"
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
    pub max_drawdown_pct: f64, // deepest drawdown in the window (negative)
    pub max_drawdown_peak_date: String,
    pub max_drawdown_trough_date: String,
    pub longest_drawdown_days: usize, // sessions from peak to recovery (or to end of window if unrecovered)
    pub corrections_5pct: usize,      // count of local-peak-to-trough declines ≥5%
    pub corrections_10pct: usize,     // count of local-peak-to-trough declines ≥10%
    pub current_drawdown_pct: f64,    // latest close vs running peak (negative or 0)
    pub regime_label: String, // "RECOVERING" | "SHALLOW" | "MEANINGFUL" | "SEVERE" | "CATASTROPHIC" | "INSUFFICIENT_DATA"
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
    pub ret_1m_pct: f64, // % change over trailing 21 sessions
    pub ret_3m_pct: f64,
    pub ret_6m_pct: f64,
    pub ret_ytd_pct: f64,    // % change from first session of as_of's year
    pub ret_1y_pct: f64,     // % change over trailing 253 sessions
    pub trend_label: String, // "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 89/90 — deferred benchmark / peer-relative parity ──

/// MOMRANK_MULTI — sector-relative percentile rank of cached PRICEPERF
/// horizons. Higher recent returns vs peers earn a higher rank, with a
/// weighted composite across 1M / 3M / 6M / YTD / 1Y returns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MomentumRankMultiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub ret_1m_pct: f64,
    pub ret_3m_pct: f64,
    pub ret_6m_pct: f64,
    pub ret_ytd_pct: f64,
    pub ret_1y_pct: f64,
    pub pct_1m: f64,
    pub pct_3m: f64,
    pub pct_6m: f64,
    pub pct_ytd: f64,
    pub pct_1y: f64,
    pub composite_percentile: f64, // weighted blend of horizon percentiles
    pub horizons_above_median: usize, // 0..=5
    pub rank_position: usize,      // 1 = strongest momentum in sector
    pub rank_label: String,        // TOP_DECILE .. BOTTOM_DECILE | INSUFFICIENT_DATA | NO_DATA
    pub note: String,
}

/// CORRSTK — rolling benchmark correlation snapshot against SPY and, when
/// available, the sector ETF benchmark. Uses intersected daily log returns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrStkSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub symbol_sector: String,
    pub market_benchmark: String, // usually "SPY"
    pub sector_benchmark: String, // e.g. "XLK", empty when unavailable
    pub overlaps_spy_20d: usize,
    pub overlaps_spy_60d: usize,
    pub overlaps_spy_252d: usize,
    pub overlaps_sector_20d: usize,
    pub overlaps_sector_60d: usize,
    pub overlaps_sector_252d: usize,
    pub corr_spy_20d: f64,
    pub corr_spy_60d: f64,
    pub corr_spy_252d: f64,
    pub beta_spy_252d: f64,
    pub r_squared_spy_252d: f64,
    pub corr_sector_20d: f64,
    pub corr_sector_60d: f64,
    pub corr_sector_252d: f64,
    pub beta_sector_252d: f64,
    pub r_squared_sector_252d: f64,
    pub dominant_benchmark: String, // "SPY" | sector ETF | "NONE"
    pub correlation_label: String, // INDEX_LOCKSTEP | SECTOR_LOCKSTEP | MIXED | DIVERGENT | INSUFFICIENT_DATA
    pub note: String,
}

/// TLRANK — 30-day trading-liquidity rank vs sector peers.
/// Percentile rank of trailing 30-session average dollar volume within the
/// same sector. Higher ADV$ = deeper near-term liquidity = higher rank.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThirtyDayLiquidityRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub window_days: i32,
    pub bars_used: usize,
    pub avg_30d_dollar_volume: f64,
    pub tier_label: String, // DEEP / LIQUID / MODERATE / THIN / ILLIQUID
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_dollar_volume: f64,
    pub sector_p25_dollar_volume: f64,
    pub sector_p75_dollar_volume: f64,
    pub percentile_rank: f64, // 0..100 (higher = deeper recent liquidity)
    pub rank_position: usize, // 1-based (1 = deepest recent liquidity)
    pub rank_label: String,   // standard decile ladder
    pub note: String,
}

/// CORRRANK — sector rank of benchmark linkage.
/// Percentile rank of 252d absolute correlation to one benchmark basis
/// (SPY or the mapped sector ETF) across same-sector peers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub benchmark_name: String, // "SPY" or sector ETF
    pub benchmark_kind: String, // "MARKET" | "SECTOR_ETF"
    pub subject_corr_252d: f64,
    pub subject_abs_corr_252d: f64,
    pub subject_beta_252d: f64,
    pub subject_r_squared_252d: f64,
    pub subject_correlation_label: String, // copied from CORRSTK
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_abs_corr_252d: f64,
    pub sector_p25_abs_corr_252d: f64,
    pub sector_p75_abs_corr_252d: f64,
    pub percentile_rank: f64, // 0..100 (higher = tighter benchmark linkage)
    pub rank_position: usize, // 1-based (1 = most benchmark-linked)
    pub rank_label: String,   // standard decile ladder
    pub note: String,
}

// ── Round 93/94 — remaining cache-backed Godel parity surfaces ──

/// OPERANK_DELTA — operating margin trend rank vs sector peers.
/// Percentile rank of `MarginsSnapshot.operating_margin_change_pct`
/// within the same sector. Higher expansion in operating margin earns a
/// higher rank.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperatingMarginDeltaRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub basis: String, // copied from MARGINS: "annual" | "quarterly"
    pub latest_period: String,
    pub operating_margin_pct: f64,
    pub operating_margin_change_pct: f64,
    pub operating_trend_label: String, // copied from MARGINS
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_change_pct: f64,
    pub sector_p25_change_pct: f64,
    pub sector_p75_change_pct: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String, // TOP_DECILE .. BOTTOM_DECILE | INSUFFICIENT_DATA | NO_DATA
    pub note: String,
}

/// DIVACC — dividend growth acceleration.
/// Tracks the latest annual dividend-growth delta vs the prior year's
/// growth rate using cached dividend-payment history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DividendAccelerationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub total_payments: usize,
    pub years_covered: usize,
    pub latest_year: i32,
    pub latest_annual_dividend: f64,
    pub latest_yoy_growth_pct: f64,
    pub prior_yoy_growth_pct: f64,
    pub acceleration_pct_pts: f64, // latest_yoy - prior_yoy
    pub recent_3y_avg_growth_pct: f64,
    pub prior_3y_avg_growth_pct: f64,
    pub acceleration_3y_avg_pct_pts: f64,
    pub consecutive_growth_years: usize,
    pub consistency_score_pct: f64,
    pub annual_rows: Vec<DivgAnnualRow>,
    pub divacc_label: String, // ACCELERATING | REACCELERATING | STABLE | DECELERATING | CUTTING | NO_HISTORY
    pub note: String,
}

/// EPSACC — EPS acceleration from cached quarterly financials.
/// Compares the latest quarterly EPS y/y growth rate against the prior
/// quarter's y/y growth rate to identify acceleration or deceleration in
/// the earnings trajectory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EpsAccelerationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub quarters_used: usize,
    pub latest_period: String,
    pub latest_eps: f64,
    pub prior_year_eps: f64,
    pub latest_yoy_growth_pct: f64,
    pub prior_yoy_growth_pct: f64,
    pub acceleration_pct_pts: f64,
    pub recent_2q_avg_yoy_growth_pct: f64,
    pub prior_2q_avg_yoy_growth_pct: f64,
    pub positive_yoy_quarters: usize,
    pub epsacc_label: String, // ACCELERATING | TURNAROUND | STABLE | DECELERATING | EARNINGS_PRESSURE | INSUFFICIENT_DATA
    pub note: String,
}

/// VRP — volatility risk premium snapshot using cached IVOL + RVCONE.
/// Pairs the current ATM implied volatility against realized-vol cone
/// levels to flag cheap/rich implied-vol regimes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolRiskPremiumSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub current_atm_iv_pct: f64,
    pub iv_rank: f64,
    pub iv_percentile: f64,
    pub iv_observation_count: usize,
    pub rv20_pct: f64,
    pub rv60_pct: f64,
    pub rv252_pct: f64,
    pub rv20_percentile: f64,
    pub rv_cone_label: String,
    pub iv_minus_rv20_pct: f64,
    pub iv_to_rv20_ratio: f64,
    pub iv_minus_rv252_pct: f64,
    pub iv_to_rv252_ratio: f64,
    pub premium_label: String, // CHEAP_IV | FAIR_IV | RICH_IV | EXTREME_RICH | INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 95 — short-interest history + trend rank ─────────────────

/// SHORTRANK_DELTA — short-interest trend rank vs sector peers.
/// Uses the change in `short_percent_of_float` over the trailing 180-day
/// window, risk-inverted so short covering (more negative delta) earns a
/// higher / safer rank.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShortInterestDeltaRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub lookback_days: i32,
    pub history_points_used: usize,
    pub history_start_date: String,
    pub history_end_date: String,
    pub latest_short_pct_of_float: f64,
    pub prior_short_pct_of_float: f64,
    pub delta_short_pct_points: f64,
    pub latest_short_ratio: f64,
    pub prior_short_ratio: f64,
    pub subject_trend_label: String, // HEAVY_COVERING | COVERING | STABLE | BUILDING | HEAVY_BUILD
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_delta_pct_pts: f64,
    pub sector_p25_delta_pct_pts: f64,
    pub sector_p75_delta_pct_pts: f64,
    pub percentile_rank: f64, // risk-inverted: lower delta -> higher / safer percentile
    pub rank_position: usize, // 1 = safest short-interest trend in sector
    pub rank_label: String,   // SAFEST_DECILE … RISKIEST_DECILE | INSUFFICIENT_DATA | NO_DATA
    pub note: String,
}

// ── Round 96 — insider ownership concentration parity ─────────────

/// INSIDERCONC — insider ownership concentration vs sector peers.
/// Estimates insider-held % from the latest known `shares_owned_after` per
/// reporter in cached INS rows, normalized by Fundamentals.shares_outstanding.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsiderConcentrationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub latest_holdings_date: String,
    pub trade_rows_used: usize,
    pub reporters_covered: usize,
    pub reporters_holding_shares: usize,
    pub shares_outstanding: f64,
    pub total_estimated_insider_shares: f64,
    pub estimated_insider_pct_held: f64,
    pub largest_reporter: String,
    pub largest_reporter_shares: f64,
    pub largest_reporter_pct_of_outstanding: f64,
    pub largest_reporter_weight_pct: f64,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_pct_held: f64,
    pub sector_p25_pct_held: f64,
    pub sector_p75_pct_held: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String, // TOP_DECILE .. BOTTOM_DECILE | INSUFFICIENT_DATA | NO_DATA
    pub note: String,
}

// ── Round 21 — beta/peg rank + HP 52wk/rvcone/calendar ──

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
    pub percentile_rank: f64, // risk-inverted: low beta → high pct
    pub rank_position: usize, // 1 = safest beta in sector
    pub rank_label: String,   // SAFEST_DECILE … RISKIEST_DECILE | INSUFFICIENT_DATA | NO_DATA
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
    pub percentile_rank: f64, // value-inverted: low PEG → high pct
    pub rank_position: usize, // 1 = best value in sector
    pub rank_label: String,   // TOP_DECILE … BOTTOM_DECILE | INSUFFICIENT_DATA | NO_DATA
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
    pub pct_from_high: f64, // (latest - high) / high × 100 — negative or 0
    pub pct_from_low: f64,  // (latest - low) / low × 100 — positive or 0
    pub range_position_pct: f64, // (latest - low) / (high - low) × 100
    pub proximity_label: String, // "AT_HIGH" | "NEAR_HIGH" | "MID_RANGE" | "NEAR_LOW" | "AT_LOW" | "INSUFFICIENT_DATA"
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
    pub rv20_pct: f64, // annualized realized vol over 20 sessions
    pub rv60_pct: f64,
    pub rv120_pct: f64,
    pub rv252_pct: f64,
    pub rv20_min_pct: f64,    // min of all rolling 20d RVs in the window
    pub rv20_median_pct: f64, // median of rolling 20d RVs
    pub rv20_max_pct: f64,    // max of rolling 20d RVs
    pub rv20_percentile: f64, // latest 20d RV percentile vs rolling distribution (0-100)
    pub cone_label: String, // "COMPRESSED" | "BELOW_AVG" | "TYPICAL" | "ELEVATED" | "EXTREME" | "INSUFFICIENT_DATA"
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
    pub mtd_pct: f64,           // current month-to-date return
    pub qtd_pct: f64,           // current quarter-to-date return
    pub ytd_pct: f64,           // current year-to-date return (calendar)
    pub prior_quarter_pct: f64, // prior calendar quarter return
    pub prior_year_pct: f64,    // prior calendar year return
    pub current_year: String,
    pub current_quarter: String, // e.g. "Q2"
    pub momentum_label: String, // "ACCELERATING" | "STEADY" | "DECELERATING" | "REVERSING" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 22 — HP return-distribution + behavior stats ──

/// RETSKEW — Return distribution skewness (third standardized moment).
/// Pure symbol-local HP stat over the trailing 253-session window of log
/// returns. Positive skew → large upside outliers; negative skew → large
/// downside outliers. Complements RVCONE (second moment) and RETKURT
/// (fourth moment) with a third-moment tail-asymmetry view.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReturnSkewnessSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize, // number of log returns used
    pub mean_log_return: f64,
    pub stdev_log_return: f64,
    pub skewness: f64,            // third standardized moment
    pub positive_return_pct: f64, // share of up-days
    pub largest_up_pct: f64,      // max log-return (×100)
    pub largest_down_pct: f64,    // min log-return (×100)
    pub skew_label: String, // "STRONG_LEFT" | "LEFT" | "SYMMETRIC" | "RIGHT" | "STRONG_RIGHT" | "INSUFFICIENT_DATA"
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
    pub excess_kurtosis: f64,        // fourth standardized moment - 3
    pub outlier_2sigma_count: usize, // count of |z| > 2 returns
    pub outlier_3sigma_count: usize, // count of |z| > 3 returns
    pub outlier_2sigma_pct: f64,     // share of |z| > 2 returns (normal ≈ 4.55%)
    pub kurt_label: String, // "PLATYKURTIC" | "NORMAL" | "MILD_FAT" | "FAT" | "EXTREME_FAT" | "INSUFFICIENT_DATA"
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
    pub pct_95_return: f64,    // 95th percentile return (as %)
    pub pct_05_return: f64,    // 5th percentile return (as %)
    pub pct_99_return: f64,    // 99th percentile return
    pub pct_01_return: f64,    // 1st percentile return
    pub tail_ratio: f64,       // pct_95 / |pct_05|
    pub tail_ratio_99_01: f64, // pct_99 / |pct_01|
    pub bias_label: String, // "DOWNSIDE_HEAVY" | "SLIGHT_DOWNSIDE" | "BALANCED" | "SLIGHT_UPSIDE" | "UPSIDE_HEAVY" | "INSUFFICIENT_DATA"
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
    pub current_run_length: i32, // positive = up run, negative = down run, 0 = flat
    pub trend_label: String, // "CHOPPY" | "MIXED" | "TRENDING" | "STRONG_TRENDING" | "INSUFFICIENT_DATA"
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
    pub avg_range_60_pct: f64,    // avg (high-low)/close × 100 over 60d
    pub avg_range_252_pct: f64,   // avg (high-low)/close × 100 over 252d
    pub latest_range_pct: f64,    // latest bar's (high-low)/close × 100
    pub compression_ratio: f64,   // 60d avg / 252d avg (1.0 = neutral)
    pub widest_range_pct: f64,    // max (high-low)/close × 100 in window
    pub narrowest_range_pct: f64, // min (high-low)/close × 100 in window
    pub range_label: String, // "TIGHT" | "COMPRESSED" | "NORMAL" | "EXPANDED" | "VERY_EXPANDED" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Godel Parity Round 23 (AUTOCOR / HURST / HITRATE / GLASYM / VOLRATIO) ──
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
    pub bars_used: usize, // number of log returns used
    pub lag1_acf: f64,    // autocorrelation at lag 1
    pub lag5_acf: f64,    // autocorrelation at lag 5
    pub lag10_acf: f64,   // autocorrelation at lag 10
    pub lag20_acf: f64,   // autocorrelation at lag 20
    pub mean_log_return: f64,
    pub regime_label: String, // "MEAN_REVERTING" | "NEUTRAL" | "MOMENTUM" | "INSUFFICIENT_DATA"
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
    pub scales_used: usize,   // number of R/S scales fit
    pub min_scale: usize,     // smallest chunk size
    pub max_scale: usize,     // largest chunk size
    pub memory_label: String, // "STRONG_MEAN_REVERT" | "MEAN_REVERT" | "RANDOM_WALK" | "PERSISTENT" | "STRONG_PERSISTENT" | "INSUFFICIENT_DATA"
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
    pub hitrate_5d: f64,   // positive share over last 5 bars
    pub hitrate_20d: f64,  // positive share over last 20 bars
    pub hitrate_60d: f64,  // positive share over last 60 bars
    pub hitrate_252d: f64, // positive share over last 252 bars
    pub up_days: usize,
    pub down_days: usize,
    pub flat_days: usize,
    pub hit_label: String, // "BEARISH" | "WEAK_BEARISH" | "NEUTRAL" | "WEAK_BULLISH" | "BULLISH" | "INSUFFICIENT_DATA"
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
    pub avg_up_pct: f64,      // mean |up-day return| (%)
    pub avg_down_pct: f64,    // mean |down-day return| (%)
    pub median_up_pct: f64,   // median |up-day return| (%)
    pub median_down_pct: f64, // median |down-day return| (%)
    pub magnitude_ratio: f64, // avg_up_pct / avg_down_pct
    pub up_days: usize,
    pub down_days: usize,
    pub asymmetry_label: String, // "DOWNSIDE_HEAVY" | "SLIGHT_DOWNSIDE" | "BALANCED" | "SLIGHT_UPSIDE" | "UPSIDE_HEAVY" | "INSUFFICIENT_DATA"
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
    pub avg_up_volume: f64,   // mean volume on up-days
    pub avg_down_volume: f64, // mean volume on down-days
    pub median_up_volume: f64,
    pub median_down_volume: f64,
    pub up_down_volume_ratio: f64, // avg_up_volume / avg_down_volume
    pub max_up_volume: f64,        // largest single up-day volume in window
    pub max_down_volume: f64,      // largest single down-day volume in window
    pub up_days: usize,
    pub down_days: usize,
    pub flow_label: String, // "DISTRIBUTION" | "SLIGHT_DISTRIBUTION" | "NEUTRAL" | "SLIGHT_ACCUMULATION" | "ACCUMULATION" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 24 — HP drawup/gap/vol-cluster/close-placement/AR(1) stats ──

/// DRAWUP — Rally history (mirror of DDHIST).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Tracks the running trough and each run from trough-to-peak: max
/// drawup, longest duration, and count of ≥5% / ≥10% rallies.
/// Complements DDHIST with the upside equivalent.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawupHistorySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub max_drawup_pct: f64, // deepest rally from a trough (positive)
    pub max_drawup_trough_date: String,
    pub max_drawup_peak_date: String,
    pub longest_drawup_days: usize, // sessions from trough to next failure or end of window
    pub rallies_5pct: usize,        // count of local-trough-to-peak advances ≥5%
    pub rallies_10pct: usize,       // count of local-trough-to-peak advances ≥10%
    pub current_drawup_pct: f64,    // latest close vs running trough (positive or 0)
    pub rally_label: String, // "MUTED" | "MILD" | "MEANINGFUL" | "STRONG" | "EXPLOSIVE" | "INSUFFICIENT_DATA"
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
    pub gap_up_count: usize,       // gap > +0.5%
    pub gap_down_count: usize,     // gap < -0.5%
    pub avg_gap_pct: f64,          // mean of all gap %s
    pub avg_gap_up_pct: f64,       // mean of up-gaps only
    pub avg_gap_down_pct: f64,     // mean of down-gaps only
    pub largest_gap_up_pct: f64,   // single largest gap up
    pub largest_gap_down_pct: f64, // single largest gap down (negative)
    pub gap_frequency_pct: f64,    // (gap_up + gap_down) / total_bars * 100
    pub bias_label: String, // "DOWN_BIAS" | "SLIGHT_DOWN" | "NEUTRAL" | "SLIGHT_UP" | "UP_BIAS" | "INSUFFICIENT_DATA"
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
    pub sq_acf_lag1: f64, // ACF of r² at lag 1
    pub sq_acf_lag5: f64,
    pub sq_acf_lag20: f64,
    pub abs_acf_lag1: f64, // ACF of |r| at lag 1
    pub abs_acf_lag5: f64,
    pub abs_acf_lag20: f64,
    pub cluster_label: String, // "NONE" | "MILD" | "MODERATE" | "STRONG" | "VERY_STRONG" | "INSUFFICIENT_DATA"
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
    pub bars_used: usize,        // bars with high > low
    pub avg_placement: f64,      // mean pos ∈ [0, 1]
    pub median_placement: f64,   // median pos ∈ [0, 1]
    pub latest_placement: f64,   // latest bar's pos
    pub pct_near_high: f64,      // % of bars with pos > 0.8
    pub pct_near_low: f64,       // % of bars with pos < 0.2
    pub placement_label: String, // "STRONG_BEAR" | "BEAR" | "NEUTRAL" | "BULL" | "STRONG_BULL" | "INSUFFICIENT_DATA"
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
    pub beta: f64,            // AR(1) slope
    pub alpha: f64,           // AR(1) intercept
    pub half_life_days: f64,  // -ln(2) / ln(|β|) for β ∈ (0, 1); else 0
    pub r_squared: f64,       // goodness-of-fit
    pub regime_label: String, // "FAST_REVERT" | "MEAN_REVERTING" | "NEUTRAL" | "PERSISTENT" | "STRONG_PERSISTENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 25 — HP downside-vol / Sharpe / efficiency / wick / vol-of-vol ──
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
    pub mean_log_return: f64,       // mean r over window
    pub downside_dev: f64,          // sqrt(mean(min(r,0)²))
    pub downside_dev_ann: f64,      // downside_dev × √252
    pub upside_dev: f64,            // sqrt(mean(max(r,0)²))
    pub sortino_ratio: f64,         // mean(r) / downside_dev
    pub sortino_ratio_ann: f64,     // (mean × 252) / downside_dev_ann
    pub downside_pct_of_total: f64, // downside_dev² / total_var × 100
    pub sortino_label: String, // "VERY_POOR" | "POOR" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
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
    pub sharpe_ratio: f64,     // raw daily
    pub sharpe_ratio_ann: f64, // × √252
    pub mean_return_ann: f64,  // mean × 252
    pub stdev_return_ann: f64, // stdev × √252
    pub sharpe_label: String, // "POOR" | "BELOW_AVG" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
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
    pub net_change: f64,          // signed end - start
    pub net_change_pct: f64,      // (end/start - 1) × 100
    pub sum_abs_changes: f64,     // Σ |close_t - close_{t-1}|
    pub efficiency_ratio: f64,    // |net| / sum_abs (signed direction separate)
    pub signed_efficiency: f64,   // efficiency_ratio × sign(net_change)
    pub efficiency_label: String, // "CHOP" | "NOISY" | "MIXED" | "TRENDING" | "STRONG_TREND" | "INSUFFICIENT_DATA"
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
    pub bars_used: usize,    // bars with high > low
    pub avg_upper_wick: f64, // mean upper wick share
    pub avg_lower_wick: f64, // mean lower wick share
    pub median_upper_wick: f64,
    pub median_lower_wick: f64,
    pub avg_body_share: f64,  // 1 - upper - lower
    pub wick_bias_score: f64, // avg_lower - avg_upper
    pub bias_label: String, // "SELLER_REJECT" | "SELLER_LEAN" | "NEUTRAL" | "BUYER_LEAN" | "BUYER_DEFEND" | "INSUFFICIENT_DATA"
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
    pub bars_used: usize, // bars with valid rv20 values
    pub mean_rv20: f64,   // mean of rolling 20d vol (daily)
    pub stdev_rv20: f64,  // stdev of rolling 20d vol
    pub min_rv20: f64,
    pub max_rv20: f64,
    pub latest_rv20: f64,
    pub cv_rv20: f64,     // stdev_rv20 / mean_rv20 (coefficient of variation)
    pub cv_label: String, // "STABLE" | "MILD" | "MODERATE" | "UNSTABLE" | "CHAOTIC" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 26 — HP calmar / ulcer / variance-ratio / amihud / jarque-bera ──

/// CALMAR — Calmar ratio: annualized return / max drawdown.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// The canonical drawdown-adjusted return metric. Reports both
/// components (annualized return, max drawdown) plus the ratio.
/// Label classifies on the Calmar ratio value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalmarRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub total_return_pct: f64,      // (last/first - 1) × 100
    pub annualized_return_pct: f64, // total × (252 / bars_used)
    pub max_drawdown_pct: f64,      // deepest peak-to-trough decline (positive number, %)
    pub calmar_ratio: f64,          // annualized_return / max_drawdown (signed)
    pub calmar_label: String, // "VERY_POOR" | "POOR" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// ULCER — Ulcer index + Martin ratio (UPI).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `ulcer_index = sqrt(mean(dd²))` where `dd = (price - peak) / peak × 100`.
/// A continuous drawdown-weighted risk measure. Martin ratio = annualized
/// return / ulcer_index, the drawdown-analogue of Sharpe.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct UlcerIndexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ulcer_index: f64,       // sqrt(mean(dd_pct²))
    pub mean_drawdown_pct: f64, // mean of running dd series (always ≤ 0)
    pub max_drawdown_pct: f64,  // deepest dd point (always ≤ 0, most negative)
    pub pct_in_drawdown: f64,   // share of bars strictly below running peak (0-100)
    pub annualized_return_pct: f64,
    pub martin_ratio: f64,   // annualized_return / ulcer_index (UPI)
    pub ulcer_label: String, // "LOW_PAIN" | "MILD" | "MODERATE" | "HIGH" | "SEVERE" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// VARRATIO — Lo-MacKinlay variance ratio at multiple horizons.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `VR(q) = Var(q-period returns) / (q × Var(1-period returns))`.
/// VR = 1 → random walk; VR > 1 → trending; VR < 1 → mean-reverting.
/// This is the formal random-walk hypothesis *test* (with z-statistics),
/// unlike HURST/AUTOCOR which are descriptive.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VarianceRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub vr_2: f64,        // VR at horizon 2
    pub vr_5: f64,        // VR at horizon 5
    pub vr_10: f64,       // VR at horizon 10
    pub vr_20: f64,       // VR at horizon 20
    pub z_stat_2: f64,    // Lo-MacKinlay z-statistic at horizon 2
    pub z_stat_5: f64,    // Lo-MacKinlay z-statistic at horizon 5
    pub rw_label: String, // "STRONG_REVERT" | "MEAN_REVERT" | "RANDOM_WALK" | "TRENDING" | "STRONG_TREND" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// AMIHUD — Amihud illiquidity ratio.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `ILLIQ = mean(|r_t| / dollar_volume_t) × 1e6`. The canonical
/// microstructure liquidity scalar — higher = less liquid = more
/// price impact per dollar traded. Uses close × volume for dollar volume.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AmihudIlliqSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,       // days with valid dollar volume > 0
    pub mean_illiq: f64,        // mean(|r| / dvol) × 1e6
    pub median_illiq: f64,      // median of daily ILLIQ × 1e6
    pub illiq_90th: f64,        // 90th percentile — worst liquidity day in 10
    pub avg_dollar_volume: f64, // average daily close × volume
    pub illiq_label: String, // "VERY_LIQUID" | "LIQUID" | "MODERATE" | "ILLIQUID" | "VERY_ILLIQUID" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// JBNORM — Jarque-Bera normality test.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `JB = (n/6)(S² + K²/4)` where S = sample skewness and K = excess
/// kurtosis. Under H₀ (normality), JB ~ χ²(2). The p-value is exact:
/// `p = exp(-JB/2)` for χ²(2). Combines RETSKEW + RETKURT into a
/// single actionable "can we reject normality?" answer.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct JarqueBeraSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub skewness: f64,        // sample skewness
    pub excess_kurtosis: f64, // sample excess kurtosis (normal = 0)
    pub jb_statistic: f64,    // (n/6)(S² + K²/4)
    pub jb_pvalue: f64,       // exp(-JB/2) for χ²(2)
    pub normal_label: String, // "NORMAL" | "MILD_DEPARTURE" | "MODERATE_DEPARTURE" | "NON_NORMAL" | "STRONGLY_NON_NORMAL" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 27 — HP omega / DFA / Burke / monthly-seas / Roll-spread ──

/// OMEGA — Omega ratio at threshold 0.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `Ω(τ) = E[max(r-τ, 0)] / E[max(τ-r, 0)]`. Uses the *full* return
/// distribution (not just mean + variance like Sharpe). At τ=0:
/// gains-sum / losses-sum (both in absolute terms). A moment-free
/// companion to SHARPR, DOWNVOL, CALMAR.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OmegaRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub gains_sum: f64,      // Σ max(r, 0) over log-returns
    pub losses_sum: f64,     // Σ max(-r, 0) over log-returns
    pub gain_days: usize,    // count of positive return days
    pub loss_days: usize,    // count of negative return days
    pub omega_ratio: f64,    // gains_sum / losses_sum
    pub win_rate_pct: f64,   // gain_days / (gain_days + loss_days) × 100
    pub omega_label: String, // "VERY_POOR" | "POOR" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// DFA — Detrended fluctuation analysis (Hurst alternative).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Profile = cumulative sum of demeaned log-returns. For each window
/// size s in a geometric grid, split the profile into non-overlapping
/// boxes, detrend each by linear fit, and RMS the residuals → F(s).
/// log-log regress F(s) ~ s yields α (Hurst exponent). α ≈ 0.5
/// uncorrelated; α > 0.5 persistent; α < 0.5 anti-persistent. Robust
/// to non-stationarity; complementary to HURST (R/S) and VARRATIO.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DetrendedFluctuationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub alpha: f64,        // DFA exponent (≈ Hurst)
    pub num_scales: usize, // distinct window sizes sampled
    pub r_squared: f64,    // quality of log-log fit
    pub dfa_label: String, // "ANTI_PERSISTENT" | "MEAN_REVERTING" | "RANDOM_WALK" | "PERSISTENT" | "STRONGLY_PERSISTENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// BURKE — Burke ratio (sum-of-squared drawdowns adjusted return).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `Burke = annualized_return / sqrt(Σ dd_i²)` over *trough events*
/// (local minima of the running drawdown series). Between CALMAR
/// (max-dd only) and ULCER (RMS of all dd), Burke weights by the
/// distinct drawdown events, emphasizing top-k worst episodes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BurkeRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub annualized_return_pct: f64,
    pub dd_event_count: usize, // distinct peak-to-trough completed drawdown episodes
    pub sum_sq_drawdowns: f64, // Σ dd_i² in pct² units
    pub worst_event_dd_pct: f64, // deepest individual episode (positive %)
    pub burke_ratio: f64,      // ann_ret / sqrt(sum_sq_drawdowns)
    pub burke_label: String, // "VERY_POOR" | "POOR" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// MONTHSEAS — monthly seasonality hit rate.
/// Pure symbol-local HP stat over *all* available bars (not just
/// trailing 253). For each calendar month (1..12), counts the number
/// of historical years where the month's close-to-close return was
/// positive, divided by the total years observed. The canonical
/// "January effect / Sell-in-May" axis — calendar-aware view that
/// no other packet surface captures.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MonthlySeasonalitySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub years_covered: usize, // number of distinct years with at least one usable month pair
    pub month_hit_pct: [f64; 12], // share of positive months per Jan..Dec (0-100)
    pub month_mean_ret_pct: [f64; 12], // mean close-to-close % return per Jan..Dec
    pub best_month_idx: usize, // 0-based index of strongest month (0=Jan)
    pub worst_month_idx: usize, // 0-based index of weakest month
    pub best_month_hit_pct: f64,
    pub worst_month_hit_pct: f64,
    pub season_label: String, // "STRONG_SEASONAL" | "MILD_SEASONAL" | "NEUTRAL" | "INCONSISTENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// ROLLSPRD — Roll's (1984) implicit bid-ask spread.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `spread = 2 · √(-Cov(Δp_t, Δp_{t-1}))` on close-to-close price
/// changes. Only valid when the first-lag price-change covariance
/// is *negative* (as bid/ask bounce implies). When covariance is
/// non-negative (trending series), Roll's model falls through to
/// INVALID_POSITIVE_COV. Microstructure companion to AMIHUD:
/// AMIHUD captures price impact per dollar; ROLLSPRD captures the
/// implicit bounce-driven effective spread in bps.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RollSpreadSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub first_lag_cov: f64,       // Cov(Δp_t, Δp_{t-1})
    pub mean_price: f64,          // mean close over window, for bps conversion
    pub implicit_spread: f64,     // 2 · √(-first_lag_cov), price units
    pub implicit_spread_bps: f64, // implicit_spread / mean_price × 1e4
    pub roll_label: String, // "TIGHT" | "NORMAL" | "WIDE" | "VERY_WIDE" | "INVALID_POSITIVE_COV" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 28 — HP range-vol / Garman-Klass / Rogers-Satchell / CVaR / dow-effect ──

/// PARKINSON — Parkinson (1980) high-low range-based volatility estimator.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `σ² = (1/(4·ln(2)·n)) · Σ (ln(H/L))²`. Uses only H and L — but
/// by virtue of being range-based is ~5.2× more statistically
/// efficient than close-to-close vol. Reported as annualized vol
/// percentage (daily σ × √252 × 100).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ParkinsonVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub daily_vol_pct: f64,      // daily σ × 100
    pub annualized_vol_pct: f64, // daily σ × √252 × 100
    pub mean_hl_log_ratio: f64,  // mean of ln(H/L) across window
    pub vol_label: String, // "VERY_LOW" | "LOW" | "NORMAL" | "HIGH" | "VERY_HIGH" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// GKVOL — Garman-Klass (1980) OHLC volatility estimator.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `σ² = (1/n) · Σ [0.5·(ln(H/L))² - (2ln2 - 1)·(ln(C/O))²]`.
/// Combines the H-L range with the C-O drift to achieve ~7.4×
/// efficiency over close-to-close. The most commonly used
/// range-based vol estimator in practice.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GarmanKlassVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub daily_vol_pct: f64,
    pub annualized_vol_pct: f64,
    pub range_component: f64, // mean 0.5·(ln H/L)²
    pub co_component: f64,    // mean (2ln2-1)·(ln C/O)²
    pub vol_label: String, // "VERY_LOW" | "LOW" | "NORMAL" | "HIGH" | "VERY_HIGH" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// RSVOL — Rogers-Satchell (1991) drift-independent OHLC volatility estimator.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// `σ² = (1/n) · Σ [ln(H/C)·ln(H/O) + ln(L/C)·ln(L/O)]`.
/// Unlike Parkinson and Garman-Klass, Rogers-Satchell is **unbiased
/// under non-zero drift** — it correctly estimates variance even
/// when the underlying series has a non-zero mean log-return per bar.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RogersSatchellVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub daily_vol_pct: f64,
    pub annualized_vol_pct: f64,
    pub vol_label: String, // 5-bucket vol label (same scheme as PARKINSON/GKVOL)
    pub note: String,
}

/// CVAR — Conditional Value-at-Risk / Expected Shortfall at 5%.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Identifies the 5th percentile of daily log returns (VaR) and
/// reports the *mean* of returns that are ≤ that threshold (ES).
/// Distinct from TAILR (which reports the quantile ratio) and
/// DOWNVOL (variance of negative returns): CVaR answers
/// "given we're in the worst 5% of days, what's the *average* loss?"
/// — the coherent downside-risk measure preferred by Basel III and
/// most modern risk frameworks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CVaRSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub var_5pct_ret_pct: f64,  // 5th percentile daily return, as %
    pub cvar_5pct_ret_pct: f64, // mean of returns ≤ VaR(5%), as %
    pub var_1pct_ret_pct: f64,  // 1st percentile daily return, as %
    pub cvar_1pct_ret_pct: f64, // mean of returns ≤ VaR(1%), as %
    pub tail_days_5pct: usize,  // count of days in the 5% tail
    pub tail_days_1pct: usize,
    pub cvar_label: String, // "MINIMAL" | "LOW" | "MODERATE" | "HIGH" | "EXTREME" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// DOWEFFECT — Day-of-week seasonality hit rate + mean return.
/// Pure symbol-local HP stat over the *full* HP cache (not 253-windowed).
/// For each weekday (Mon-Fri) reports hit rate (share of that weekday
/// which closed positive intraday, O→C) and mean intraday return %.
/// Calendar companion to MONTHSEAS: captures Monday-effect, Friday-rally,
/// Wednesday-weakness etc. that only a day-of-week lens can see.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DayOfWeekEffectSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub weeks_covered: usize,
    pub dow_hit_pct: [f64; 5], // hit rate per Mon..Fri, share of positive O→C
    pub dow_mean_ret_pct: [f64; 5], // mean intraday % return per Mon..Fri
    pub dow_sample_count: [usize; 5], // count of samples per weekday
    pub best_dow_idx: usize,   // 0=Mon..4=Fri
    pub worst_dow_idx: usize,
    pub best_dow_hit_pct: f64,
    pub worst_dow_hit_pct: f64,
    pub dow_label: String, // "STRONG_EFFECT" | "MILD_EFFECT" | "NEUTRAL" | "INCONSISTENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 29 — HP Sterling / Kelly / Ljung-Box / runs test / zero-return ──

/// STERLING — Sterling ratio: annualized return / average of N worst drawdowns.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Distinct drawdown ratio family completion: CALMAR uses only max-dd,
/// ULCER is RMS of all continuous dd points, BURKE sums squared event
/// drawdowns — STERLING uses the *arithmetic mean* of the top-N (default 5)
/// worst distinct drawdown events. This gives the most directly interpretable
/// "average of my worst N drawdowns" reading.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SterlingRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub annualized_return_pct: f64,
    pub worst_n: usize,         // N worst distinct dd events used (default 5)
    pub dd_event_count: usize,  // total distinct dd events in window
    pub mean_worst_dd_pct: f64, // mean of worst N event drawdowns, as %
    pub sterling_ratio: f64,    // annualized_return / mean_worst_dd (magnitudes)
    pub sterling_label: String, // "VERY_POOR" | "POOR" | "NEUTRAL" | "GOOD" | "EXCELLENT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// KELLYF — Kelly fraction / optimal leverage.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Classical position-sizing scalar: `f* = (b·p − q) / b` where
/// p=win rate, q=1−p, b=avg_win/avg_loss. First packet surface in the
/// position-sizing axis — SHARPR/DOWNVOL/CALMAR etc all measure
/// realized risk-adjusted performance; KELLYF gives a forward-looking
/// optimal-stake scalar derived from the same return distribution.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KellyFractionSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub win_rate: f64,       // p, fraction of positive-return days
    pub loss_rate: f64,      // q = 1 − p (positive-return days exclude zero)
    pub avg_win_pct: f64,    // mean of positive daily returns (%), 0 if none
    pub avg_loss_pct: f64,   // mean of |negative| daily returns (%), 0 if none
    pub win_loss_ratio: f64, // b = avg_win / avg_loss, ∞-handling emits 0.0
    pub kelly_fraction: f64, // f* = (b·p − q) / b; can be negative (skip) or capped
    pub half_kelly: f64,     // kelly/2, conservative practitioner default
    pub kelly_label: String, // "SKIP" | "MARGINAL" | "MODERATE" | "AGGRESSIVE" | "ALL_IN" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// LJUNGB — Ljung-Box Q-statistic at lag 10 (portmanteau autocorrelation test).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Complements AUTOCOR (which reports individual-lag ACFs at 1/5/10/20)
/// with a *joint* test: `Q = n(n+2) · Σ(ρ_k²/(n−k))` for k=1..h, with
/// `Q ~ χ²(h)` under the null. Gives a single combined-lag p-value for
/// the "returns are white noise" hypothesis — the canonical
/// econometrics test for model adequacy.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LjungBoxSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub lag_h: usize,             // h, usually 10
    pub q_statistic: f64,         // Q
    pub p_value: f64,             // P(χ²(h) ≥ Q)
    pub reject_white_noise: bool, // p < 0.05
    pub ljungb_label: String, // "WHITE_NOISE" | "WEAK_DEP" | "MODERATE_DEP" | "STRONG_DEP" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// RUNSTEST — Wald-Wolfowitz runs test for randomness of the sign sequence.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Given the sequence of positive/negative daily returns, counts the
/// number of runs (blocks of consecutive same-signed returns) and
/// compares to its null distribution (mean = 2n₁n₂/n + 1,
/// variance = 2n₁n₂(2n₁n₂−n) / (n²(n−1))). Distinct from RUNLEN, which
/// is descriptive (longest/mean streak); RUNSTEST is inferential (z-stat +
/// p-value against the "sign sequence is random" null).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunsTestSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub positive_days: usize,
    pub negative_days: usize,
    pub runs_observed: usize,
    pub runs_expected: f64,
    pub runs_std: f64,
    pub z_statistic: f64,
    pub p_value: f64,            // two-sided
    pub reject_randomness: bool, // p < 0.05
    pub runs_label: String, // "RANDOM" | "SLIGHT_CLUST" | "MOD_CLUST" | "STRONG_CLUST" | "ANTI_CLUST" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// ZERORET — Lesmond-Ogden-Trzcinka zero-return-day fraction.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Fraction of bars with |log_return| < epsilon (default 1e-6, i.e.
/// exactly unchanged close) as a canonical academic liquidity proxy:
/// illiquid securities show more zero-return days (dealers don't update
/// the close because nobody traded). Distinct from AMIHUD (price
/// impact per $) and ROLLSPRD (implicit bid-ask spread) — ZERORET is
/// the third foundational microstructure scalar.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ZeroReturnSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub zero_day_count: usize,
    pub zero_day_pct: f64,          // 100 * zero_count / bars_used
    pub longest_zero_streak: usize, // longest run of consecutive zero-return bars
    pub epsilon: f64,               // threshold used (default 1e-6)
    pub zero_label: String, // "HIGHLY_LIQUID" | "LIQUID" | "MODERATE" | "ILLIQUID" | "VERY_ILLIQUID" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── Round 30: PSR / ADF / MNKENDALL / BIPOWER / DDDUR ──────────────

/// PSR — Probabilistic Sharpe Ratio (Lopez de Prado 2012).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Addresses the well-known critique that the classical Sharpe
/// ratio assumes normally-distributed returns. PSR(SR*) is the
/// *probability* that the true Sharpe exceeds some benchmark SR*,
/// computed as
/// `PSR = Φ((SR − SR*)·√(n−1) / √(1 − γ₃·SR + (γ₄−1)/4·SR²))`
/// where γ₃ = sample skewness, γ₄ = sample kurtosis (not excess).
/// Higher PSR at SR*=0 means the positive Sharpe is unlikely to be
/// a sampling fluke. First packet surface to correct a return-
/// quality ratio for higher-order moments.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProbabilisticSharpeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub sharpe: f64,       // observed annualized Sharpe (same convention as SHARPR)
    pub skewness: f64,     // sample γ₃
    pub kurtosis: f64,     // sample γ₄ (not excess)
    pub sr_benchmark: f64, // SR* used (default 0)
    pub psr: f64,          // PSR(SR*) ∈ [0, 1]
    pub psr_label: String, // "VERY_LOW" <0.50 / "LOW" <0.75 / "MODERATE" <0.90 / "HIGH" <0.95 / "VERY_HIGH" ≥0.95 / INSUFFICIENT_DATA
    pub note: String,
}

/// ADF — Augmented Dickey-Fuller unit-root / stationarity test.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Applied to **log prices** (not returns). Regresses
/// `Δlog(p)_t = α + β·log(p)_{t-1} + ε` and reports
/// `t-stat = β̂ / se(β̂)` against Dickey-Fuller critical values
/// (MacKinnon 1996 approximation). Rejection of H₀ (β=0) means
/// the log-price series is stationary. Complements Hurst (long-
/// memory exponent) and DFA (nonstationarity-robust persistence)
/// with a formal unit-root hypothesis test. Note: this is the
/// zero-lag DF test, not the augmented form — the lag-0 variant
/// is standard in trading literature.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DickeyFullerSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub beta: f64,              // OLS slope on lagged log-price
    pub se_beta: f64,           // standard error of β̂
    pub t_statistic: f64,       // β̂ / se(β̂)
    pub crit_1pct: f64,         // -3.43 (constant-only)
    pub crit_5pct: f64,         // -2.86
    pub crit_10pct: f64,        // -2.57
    pub reject_unit_root: bool, // t < crit_5pct
    pub adf_label: String, // "STATIONARY" / "BORDERLINE" / "NON_STATIONARY" / INSUFFICIENT_DATA
    pub note: String,
}

/// MNKENDALL — Mann-Kendall nonparametric trend test.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Applied to log prices. Counts sign(x_j − x_i) over all i<j to
/// form `S = Σᵢ<ⱼ sign(x_j − x_i)`. Under H₀ (no trend), S is
/// approximately normal with mean 0 and variance
/// `n(n−1)(2n+5)/18` (no ties correction). Z-statistic and
/// two-sided p-value via standard normal CDF. Distribution-free
/// (does not assume linearity or normality) — complements
/// Hurst/DFA (persistence) and ADF (stationarity) with a
/// formal trend-presence test.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MannKendallSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub s_statistic: i64, // Kendall S sum
    pub variance: f64,    // Var(S) under null
    pub z_statistic: f64,
    pub p_value: f64,          // two-sided
    pub tau: f64,              // Kendall τ = S / (n·(n-1)/2)
    pub reject_no_trend: bool, // p < 0.05
    pub mk_label: String, // "STRONG_UP" / "UP" / "NO_TREND" / "DOWN" / "STRONG_DOWN" / INSUFFICIENT_DATA
    pub note: String,
}

/// BIPOWER — Bipower variation and realized-jump ratio.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Barndorff-Nielsen & Shephard (2004) decomposition: the realized
/// variance `RV = Σr_t²` includes both continuous (diffusive)
/// volatility and jumps. Bipower variation
/// `BPV = (π/2)·Σ|r_t|·|r_{t-1}|` converges to the integrated
/// variance of the continuous component *only*, under mild
/// conditions. Jump ratio `1 − BPV/RV` ∈ [0, 1] estimates the
/// share of realized variance attributable to jumps. Large
/// jump ratio ⇒ returns are dominated by discrete events;
/// small ⇒ classic diffusive behaviour. Distinct from the
/// vol-level estimators (CLOSEVOL/PARKINSON/GKVOL/RSVOL) — this
/// is a *composition* metric, not a magnitude.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BipowerVariationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub realized_var: f64,           // Σ r_t²
    pub bipower_var: f64,            // BPV
    pub continuous_vol_ann_pct: f64, // √(BPV · 252/n) × 100
    pub realized_vol_ann_pct: f64,   // √(RV · 252/n) × 100 (close-to-close annualized)
    pub jump_ratio: f64,             // max(0, 1 − BPV/RV), clamped to [0, 1]
    pub jump_pct: f64,               // 100 × jump_ratio
    pub jump_label: String, // "NO_JUMPS" <0.05 / "MILD_JUMPS" <0.20 / "NOTABLE_JUMPS" <0.40 / "HEAVY_JUMPS" ≥0.40 / INSUFFICIENT_DATA
    pub note: String,
}

/// DDDUR — Drawdown duration statistics.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Walks the closing-price series with a running-max tracker and
/// records, for each *closed* drawdown event, the number of bars
/// from peak to recovery. Complements the *magnitude*-focused
/// CALMAR (single worst dd) / BURKE (sum-of-squares) / STERLING
/// (mean of N worst) family with a *duration* axis: "how long
/// am I underwater?" Reports max/mean/median event durations,
/// total bars underwater in the window, % of time underwater, and
/// (if a drawdown is still open at window end) a `currently_underwater`
/// flag with `current_dd_duration`.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawdownDurationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub dd_event_count: usize, // closed drawdowns recorded
    pub max_dd_duration_bars: usize,
    pub mean_dd_duration_bars: f64,
    pub median_dd_duration_bars: f64,
    pub total_bars_underwater: usize, // including ongoing
    pub pct_time_underwater: f64,     // 100 × total / bars_used
    pub currently_underwater: bool,
    pub current_dd_duration_bars: usize,
    pub dddur_label: String, // "MOSTLY_DRY" <20% / "FREQUENT_DD" <40% / "PERSISTENT_DD" <60% / "DEEP_WATER" ≥60% / INSUFFICIENT_DATA
    pub note: String,
}

/// HILLTAIL — Hill tail-index estimator.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// For order statistics X_(1) ≥ X_(2) ≥ … ≥ X_(n) of |r_t|, the
/// Hill estimator `α̂ = k / Σᵢ₌₁ᵏ log(X_(i) / X_(k+1))` estimates
/// the Pareto-tail index assuming `P(|R| > x) ≈ c·x^(−α)`. Small α
/// ⇒ heavy power-law tails (α ≤ 2 ⇒ infinite variance in the
/// underlying Pareto); large α ⇒ tails decay fast (α > 4 ≈ Gaussian-
/// like). Complements JBNORM (joint normality *test*) and KURT
/// (fourth-moment magnitude) with a *nonparametric power-law
/// exponent*. Separate estimates on left-tail (negative-return
/// magnitudes) and right-tail (positive-return magnitudes) expose
/// tail asymmetry invisible to KURT.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HillTailSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub k_order_stats: usize,  // top-k used (10% of n, min 10)
    pub threshold_abs: f64,    // X_(k+1) from |r| ordering
    pub hill_alpha_abs: f64,   // tail index on |r|
    pub hill_alpha_left: f64,  // tail index on negative-return magnitudes
    pub hill_alpha_right: f64, // tail index on positive-return magnitudes
    pub tail_label: String, // "GAUSSIAN_LIKE" α>4 / "LIGHT_TAIL" α>3 / "MODERATE_TAIL" α>2 / "HEAVY_TAIL" α>1 / "VERY_HEAVY_TAIL" α≤1 / INSUFFICIENT_DATA
    pub note: String,
}

/// ARCHLM — Engle (1982) ARCH Lagrange-multiplier test.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Regresses squared mean-residuals ε_t² on intercept +
/// ε²_{t-1}, …, ε²_{t-q} (here q=5) and reports
/// `LM = n·R² ~ χ²(q)` under H₀ (no conditional heteroskedasticity).
/// Critical values χ²₀.₀₅(5)=11.07, χ²₀.₀₁(5)=15.09 (hardcoded).
/// Complements VOLOFVOL (descriptive rolling-σ scatter) with a
/// formal hypothesis test for volatility clustering, which is the
/// canonical stylized fact of financial returns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ArchLmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub q_lags: usize,              // 5
    pub r_squared: f64,             // R² of ε² regression
    pub lm_statistic: f64,          // n·R²
    pub p_value: f64,               // via Wilson-Hilferty transform to Φ
    pub crit_5pct_chi2: f64,        // 11.07
    pub crit_1pct_chi2: f64,        // 15.09
    pub reject_homoskedastic: bool, // LM > crit_5pct_chi2
    pub arch_label: String,         // "NO_ARCH" / "WEAK_ARCH" / "STRONG_ARCH" / INSUFFICIENT_DATA
    pub note: String,
}

/// PAINRATIO — Pain index and pain ratio (Zephyr/FIBA).
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Pain Index = arithmetic mean of |dd_t| (%) across every bar of
/// the window, where dd_t = (close_t − peak_t)/peak_t · 100.
/// Pain Ratio = annualized_return / pain_index — the drawdown-
/// averaged analogue of Sharpe/Calmar/Burke/Ulcer/Sterling. Distinct
/// denominators: CALMAR=max, BURKE=√Σdd², STERLING=mean of worst N,
/// ULCER=√mean(dd²) (RMS), PAIN=mean|dd| (L¹). Pain treats every
/// bar equally, not just the worst ones.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PainRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pain_index_pct: f64, // mean |dd_t| in %
    pub annualized_return_pct: f64,
    pub pain_ratio: f64,    // ann_return / pain_index
    pub max_dd_pct: f64,    // companion magnitude
    pub pain_label: String, // "LOW_PAIN" <1 / "MILD_PAIN" <3 / "MODERATE_PAIN" <7 / "HIGH_PAIN" <15 / "SEVERE_PAIN" ≥15 / INSUFFICIENT_DATA
    pub note: String,
}

/// CUSUM — Brown-Durbin-Evans (1975) OLS CUSUM test for mean stability.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Builds standardized cumulative sum
/// `S_t = Σ_{s=1..t} (r_s − r̄) / σ̂` and reports the scaled
/// test statistic `D = max_t |S_t| / √n`, which under H₀ (mean
/// stability) has the Kolmogorov-Smirnov limiting distribution with
/// critical values {10%=1.22, 5%=1.36, 1%=1.63}. Rejection signals
/// a structural break in the return mean. Pairs with ADF
/// (stationarity of levels), LJUNGB (joint autocorrelation), and
/// RUNSTEST (randomness of signs) as the fourth inferential
/// diagnostic and the first structural-break test in the packet.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CusumBreakSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub max_abs_cusum: f64,       // max_t |S_t|
    pub test_statistic: f64,      // max_abs_cusum / √n
    pub max_abs_bar: usize,       // index t where max is achieved
    pub direction_at_max: String, // "UP" if S_t>0 at max, "DOWN" if <0, "NONE" otherwise
    pub crit_10pct: f64,          // 1.22
    pub crit_5pct: f64,           // 1.36
    pub crit_1pct: f64,           // 1.63
    pub reject_stability: bool,   // test_statistic > crit_5pct
    pub cusum_label: String, // "STABLE" / "MARGINAL" / "BREAK_DETECTED" / "STRONG_BREAK" / INSUFFICIENT_DATA
    pub note: String,
}

/// CFVAR — Cornish-Fisher modified Value-at-Risk.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Applies the Cornish-Fisher (1938) expansion
/// `z* = z + (z²−1)·γ₃/6 + (z³−3z)·γ₄/24 − (2z³−5z)·γ₃²/36`
/// to the standard-normal quantile, then reports
/// `CF-VaR = μ + z*·σ`. This corrects the Gaussian VaR quantile
/// for sample skewness (γ₃) and *excess* kurtosis (γ₄). Complements
/// historical CVAR (fully nonparametric tail) with a parametric
/// skew/kurt-aware VaR, useful when an agent wants a smooth
/// analytical quantile rather than an empirical one. Reports both
/// the Gaussian and CF-adjusted quantiles at 5% and 1%, and the
/// dominance of skew-term vs kurt-term in driving any deviation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CornishFisherSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mean_ret_pct: f64,           // daily mean × 100
    pub sigma_ret_pct: f64,          // daily std × 100
    pub skewness: f64,               // γ₃
    pub excess_kurtosis: f64,        // γ₄ (excess — subtract 3)
    pub gauss_var_5pct_pct: f64,     // μ + (−1.645)·σ × 100
    pub cf_var_5pct_pct: f64,        // μ + z*(5%)·σ × 100
    pub gauss_var_1pct_pct: f64,     // μ + (−2.326)·σ × 100
    pub cf_var_1pct_pct: f64,        // μ + z*(1%)·σ × 100
    pub cf_adjustment_5pct_pct: f64, // cf_var_5pct − gauss_var_5pct
    pub skew_term_5pct: f64,         // (z²−1)·γ₃/6 − (2z³−5z)·γ₃²/36 at z=-1.645
    pub kurt_term_5pct: f64,         // (z³−3z)·γ₄/24 at z=-1.645
    pub cfvar_label: String, // "BENIGN" / "SKEW_DRIVEN" / "KURT_DRIVEN" / "EXTREME_DEVIATION" / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 32 structs ──────────────────────────────────────────────

/// ENTROPY — Shannon entropy of the return distribution.
/// H = −Σ pᵢ log₂(pᵢ) over a histogram of daily log-returns
/// (bins = ceil(√n)). Low H → concentrated/predictable returns;
/// high H → dispersed/unpredictable.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct EntropySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub num_bins: usize,
    pub entropy_bits: f64,       // H in bits (log base 2)
    pub max_entropy_bits: f64,   // log₂(num_bins) — uniform distribution
    pub normalised_entropy: f64, // H / H_max ∈ [0,1]
    pub entropy_label: String, // LOW_ENTROPY / MODERATE_ENTROPY / HIGH_ENTROPY / VERY_HIGH_ENTROPY / INSUFFICIENT_DATA
    pub note: String,
}

/// RACHEV — Rachev ratio = ES_α(+R) / ES_α(−R).
/// Compares right-tail expected gain to left-tail expected loss
/// at matching confidence levels (5% and 1%). Rachev > 1 ⇒
/// upside tail outweighs downside tail.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct RachevSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub es_right_5pct: f64, // mean of top 5% returns (×100 ⇒ pct)
    pub es_left_5pct: f64,  // mean of bottom 5% returns (×100 ⇒ pct, negative)
    pub rachev_5pct: f64,   // |es_right_5pct| / |es_left_5pct|
    pub es_right_1pct: f64,
    pub es_left_1pct: f64,
    pub rachev_1pct: f64,
    pub rachev_label: String, // STRONG_LEFT_TAIL / LEFT_HEAVY / SYMMETRIC / RIGHT_HEAVY / STRONG_RIGHT_TAIL / INSUFFICIENT_DATA
    pub note: String,
}

/// GPR — Gain-to-Pain Ratio (Schwager).
/// GPR = Σ rₜ / Σ |min(rₜ, 0)|. Also reports Profit Factor =
/// Σ max(rₜ,0) / Σ |min(rₜ, 0)| = GPR + 1.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GprSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub sum_all_returns_pct: f64, // Σ rₜ × 100
    pub sum_losses_pct: f64,      // Σ |min(rₜ, 0)| × 100 (positive number)
    pub sum_gains_pct: f64,       // Σ max(rₜ, 0) × 100
    pub gain_to_pain: f64,        // GPR = sum_all / sum_losses
    pub profit_factor: f64,       // PF = sum_gains / sum_losses = GPR + 1
    pub win_count: usize,
    pub loss_count: usize,
    pub gpr_label: String, // DEEP_PAIN / NEGATIVE / MODEST / GOOD / EXCELLENT / INSUFFICIENT_DATA
    pub note: String,
}

/// PACF — Partial autocorrelation function at lags 1-5.
/// Uses the Durbin-Levinson recursion to compute PACF from
/// the sample autocorrelation function. Reports individual lag
/// values plus Bartlett 95% critical band ±1.96/√n.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct PacfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pacf_lag1: f64,
    pub pacf_lag2: f64,
    pub pacf_lag3: f64,
    pub pacf_lag4: f64,
    pub pacf_lag5: f64,
    pub bartlett_crit_95: f64,   // ±1.96/√n
    pub significant_lags: usize, // count of lags where |PACF| > crit
    pub max_abs_pacf: f64,
    pub max_abs_lag: usize, // lag number of max |PACF|
    pub pacf_label: String, // NO_STRUCTURE / LAG1_DOMINANT / LAG_STRUCTURE / STRONG_STRUCTURE / INSUFFICIENT_DATA
    pub note: String,
}

/// APEN — Approximate entropy (Pincus 1991).
/// Measures regularity/predictability of a time series.
/// Low ApEn → regular, self-similar patterns; high ApEn →
/// irregular, complex dynamics. Parameters: m=2, r=0.2·σ.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct ApenSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize,   // m (always 2)
    pub tolerance: f64,     // r = 0.2 × std
    pub phi_m: f64,         // Φ^m(r)
    pub phi_m1: f64,        // Φ^{m+1}(r)
    pub apen: f64,          // Φ^m − Φ^{m+1}
    pub apen_label: String, // REGULAR / MODERATE / COMPLEX / HIGHLY_COMPLEX / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 33 structs ──────────────────────────────────────────────

/// UPR — Upside Potential Ratio (Sortino & van der Meer 1991).
/// UPR = E[max(r−MAR,0)] / √E[min(r−MAR,0)²] where MAR=0.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct UprSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub upm1: f64,         // upper partial moment of order 1
    pub lpm2: f64,         // lower partial moment of order 2
    pub downside_dev: f64, // √LPM₂
    pub upr: f64,          // UPM₁ / downside_dev
    pub upr_label: String, // LOW_UPSIDE / MODERATE_UPSIDE / BALANCED / HIGH_UPSIDE / VERY_HIGH_UPSIDE / INSUFFICIENT_DATA
    pub note: String,
}

/// LEVEREFF — Leverage effect (Black 1976, Christie 1982).
/// Measures asymmetric volatility: negative returns tend to
/// increase future volatility more than positive returns.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct LeverEffSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub corr_r_nextsq: f64,      // corr(rₜ, rₜ₊₁²)
    pub mean_vol_after_neg: f64, // mean |rₜ₊₁| after rₜ < 0 (×100)
    pub mean_vol_after_pos: f64, // mean |rₜ₊₁| after rₜ > 0 (×100)
    pub asym_ratio: f64,         // mean_vol_after_neg / mean_vol_after_pos
    pub lever_label: String, // STRONG_LEVERAGE / MILD_LEVERAGE / SYMMETRIC / REVERSE_LEVERAGE / INSUFFICIENT_DATA
    pub note: String,
}

/// DRAWDAR — Drawdown-at-Risk + Conditional DaR.
/// Quantile-based drawdown risk measure (Chekhlov et al. 2005).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct DrawDaRSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub dar_5pct: f64,         // 95th percentile of drawdown distribution (%)
    pub cdar_5pct: f64,        // mean dd given dd > DaR(5%) — conditional DaR (%)
    pub dar_1pct: f64,         // 99th percentile (%)
    pub cdar_1pct: f64,        // conditional DaR at 1% (%)
    pub max_dd_pct: f64,       // max drawdown (%)
    pub mean_dd_pct: f64,      // mean of all non-zero drawdowns (%)
    pub drawdar_label: String, // LOW_DD_RISK / MODERATE_DD_RISK / HIGH_DD_RISK / SEVERE_DD_RISK / INSUFFICIENT_DATA
    pub note: String,
}

/// VARHALF — Volatility half-life (vol persistence).
/// AR(1) on rolling 20d realized vol → half-life = −ln(2)/ln(β).
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct VarHalfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub vol_obs: usize,        // number of rolling-vol observations
    pub ar1_beta: f64,         // AR(1) coefficient
    pub ar1_alpha: f64,        // AR(1) intercept
    pub ar1_r2: f64,           // R² of AR(1) fit
    pub half_life_days: f64,   // −ln(2)/ln(β)
    pub varhalf_label: String, // FAST_REVERT / MODERATE_PERSIST / SLOW_PERSIST / VERY_PERSISTENT / INSUFFICIENT_DATA
    pub note: String,
}

/// GINI — Gini coefficient of |returns|.
/// Measures concentration/inequality of absolute return magnitudes.
/// Gini = 0 → all |returns| equal; Gini = 1 → one return dominates.
#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct GiniSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub gini_coeff: f64,            // Gini on |returns| ∈ [0,1]
    pub mean_abs_return_pct: f64,   // mean |r| × 100
    pub median_abs_return_pct: f64, // median |r| × 100
    pub gini_label: String, // LOW_CONCENTRATION / MODERATE_CONCENTRATION / HIGH_CONCENTRATION / VERY_HIGH_CONCENTRATION / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 34 structs ──

/// SAMPEN — Sample Entropy (Richman & Moorman 2000).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SampenSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize,     // m (default 2)
    pub tolerance: f64,       // r (default 0.2·σ)
    pub a_count: usize,       // template matches length m+1 (excl self)
    pub b_count: usize,       // template matches length m (excl self)
    pub sampen: f64,          // −ln(A/B)
    pub sampen_label: String, // REGULAR / MODERATE / COMPLEX / HIGHLY_COMPLEX / INSUFFICIENT_DATA / UNDEFINED
    pub note: String,
}

/// PERMEN — Permutation Entropy (Bandt & Pompe 2002).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PermenSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize,         // m (default 3 → 6 ordinal patterns)
    pub patterns_observed: usize, // distinct ordinal patterns seen
    pub patterns_possible: usize, // m! = 6
    pub permen_raw: f64,          // Shannon entropy of pattern distribution
    pub permen_normalised: f64,   // H / log₂(m!) ∈ [0,1]
    pub permen_label: String, // REGULAR / MODERATE / COMPLEX / HIGHLY_COMPLEX / INSUFFICIENT_DATA
    pub note: String,
}

/// RECFACT — Recovery Factor.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RecfactSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub cum_return_pct: f64,   // (last/first − 1) × 100
    pub max_drawdown_pct: f64, // absolute max drawdown × 100
    pub recovery_factor: f64,  // cum_return / |max_drawdown|
    pub recfact_label: String, // DEEP_LOSS / NEGATIVE / RECOVERING / GOOD / EXCELLENT / INSUFFICIENT_DATA
    pub note: String,
}

/// KPSS — Kwiatkowski-Phillips-Schmidt-Shin stationarity test.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KpssSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub kpss_stat: f64,          // η_μ = Σ S_t² / (n² · s²_ℓ)
    pub lag_truncation: usize,   // ℓ = floor(4·(n/100)^(2/9))
    pub crit_10: f64,            // 0.347
    pub crit_5: f64,             // 0.463
    pub crit_1: f64,             // 0.739
    pub reject_stationary: bool, // true if η_μ > crit_5
    pub kpss_label: String, // STATIONARY / WEAKLY_NONSTATIONARY / NONSTATIONARY / INSUFFICIENT_DATA
    pub note: String,
}

/// SPECENT — Spectral Entropy via DFT.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SpecentSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub num_freqs: usize,           // N/2 frequency bins
    pub spectral_entropy_raw: f64,  // −Σ pₖ log₂(pₖ) on normalised PSD
    pub spectral_entropy_norm: f64, // H / log₂(N/2) ∈ [0,1]
    pub peak_freq_idx: usize,       // index of max PSD bin
    pub peak_power_share: f64,      // fraction of total power at peak
    pub specent_label: String, // PERIODIC / MODERATE_PERIODICITY / BROAD_SPECTRUM / NOISE_LIKE / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 35 structs ──

/// ROBVOL — Robust Volatility (MAD + IQR, outlier-resistant).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RobVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub classical_sigma: f64, // standard sample σ (annualised ×√252)
    pub mad_sigma: f64,       // MAD/0.6745 (annualised)
    pub iqr_sigma: f64,       // IQR/1.349 (annualised)
    pub mad_ratio: f64,       // mad_sigma / classical_sigma
    pub iqr_ratio: f64,       // iqr_sigma / classical_sigma
    pub robvol_label: String, // HEAVY_OUTLIERS / MODERATE_OUTLIERS / CLEAN / LIGHT_TAILS / INSUFFICIENT_DATA
    pub note: String,
}

/// RENYIENT — Rényi Entropy at α=2 (collision entropy).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RenyientSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub num_bins: usize,        // histogram bins (Sturges/Freedman-Diaconis)
    pub alpha: f64,             // 2.0
    pub renyi_raw: f64,         // −log₂(Σ pᵢ²)
    pub renyi_normalised: f64,  // H₂ / log₂(K) ∈ [0,1]
    pub collision_prob: f64,    // Σ pᵢ² — probability two samples share bin
    pub renyient_label: String, // CONCENTRATED / MODERATE / DISPERSED / HIGHLY_DISPERSED / INSUFFICIENT_DATA
    pub note: String,
}

/// RETQUANT — Return Quantile Profile (9-point P1..P99).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RetquantSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub p01_pct: f64,
    pub p05_pct: f64,
    pub p10_pct: f64,
    pub p25_pct: f64,
    pub p50_pct: f64,
    pub p75_pct: f64,
    pub p90_pct: f64,
    pub p95_pct: f64,
    pub p99_pct: f64,
    pub iqr_pct: f64,           // P75 − P25
    pub tail_asymmetry: f64,    // (P99 + P01) / (P99 − P01) — +ve ⇒ right-skew tails
    pub retquant_label: String, // LEFT_TAIL_HEAVY / SYMMETRIC / RIGHT_TAIL_HEAVY / WIDE_IQR / INSUFFICIENT_DATA
    pub note: String,
}

/// MSENT — Multiscale Entropy (Costa, Goldberger, Peng 2005).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MsentSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize, // m (default 2)
    pub tolerance: f64,   // r (default 0.2·σ of raw series)
    pub max_scale: usize, // τ_max = 5
    pub sampen_scale1: f64,
    pub sampen_scale2: f64,
    pub sampen_scale3: f64,
    pub sampen_scale4: f64,
    pub sampen_scale5: f64,
    pub msent_complexity_index: f64, // Σ SampEn(τ) — integrated complexity
    pub msent_label: String, // LONG_RANGE_REGULAR / DECAYING / SUSTAINED / INCREASING / INSUFFICIENT_DATA
    pub note: String,
}

/// EWMAVOL — RiskMetrics EWMA Volatility (λ=0.94).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct EwmaVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub lambda: f64,                 // decay factor (0.94 standard)
    pub ewma_variance: f64,          // final σ²_t
    pub ewma_sigma_daily: f64,       // √variance (daily)
    pub ewma_sigma_annual: f64,      // ×√252
    pub classical_sigma_annual: f64, // sample σ ×√252 for comparison
    pub ewma_to_classical: f64,      // ewma / classical — >1 ⇒ recent vol elevated
    pub ewmavol_label: String,       // ELEVATED / NORMAL / SUPPRESSED / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 36 ──────────────────────────────────────────────────────

/// KSNORM — Kolmogorov-Smirnov normality test (standardised returns vs N(0,1)).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KsnormSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ks_statistic: f64,   // max |F_emp − Φ|
    pub critical_10pct: f64, // 1.22/√n
    pub critical_5pct: f64,  // 1.36/√n
    pub critical_1pct: f64,  // 1.63/√n
    pub reject_10pct: bool,
    pub reject_5pct: bool,
    pub reject_1pct: bool,
    pub mean: f64,            // sample mean (standardisation)
    pub sigma: f64,           // sample σ (standardisation)
    pub ksnorm_label: String, // NORMAL / MILD_DEVIATION / MODERATE_DEVIATION / STRONG_NON_NORMAL / INSUFFICIENT_DATA
    pub note: String,
}

/// ADTEST — Anderson-Darling normality test (tail-weighted, more powerful than KS).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AdtestSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ad_statistic: f64,   // A²
    pub ad_adjusted: f64,    // A²·(1 + 0.75/n + 2.25/n²)
    pub p_value_approx: f64, // Stephens approximation
    pub critical_10pct: f64, // 0.631
    pub critical_5pct: f64,  // 0.752
    pub critical_1pct: f64,  // 1.035
    pub reject_10pct: bool,
    pub reject_5pct: bool,
    pub reject_1pct: bool,
    pub adtest_label: String, // NORMAL / MILD_DEVIATION / MODERATE_DEVIATION / STRONG_NON_NORMAL / INSUFFICIENT_DATA
    pub note: String,
}

/// LMOM — Hosking 1990 L-moments (robust alternatives to classical moments).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LmomSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub l1_mean: f64,       // λ₁ = sample mean
    pub l2_scale: f64,      // λ₂ = 0.5·E|X₁−X₂|
    pub l3: f64,            // λ₃ (third L-moment)
    pub l4: f64,            // λ₄ (fourth L-moment)
    pub tau3_skew: f64,     // λ₃/λ₂ ∈ [−1,1]
    pub tau4_kurt: f64,     // λ₄/λ₂ ∈ [−0.25, 1]
    pub lmom_label: String, // HEAVY_LEFT / HEAVY_RIGHT / HEAVY_TAILS / LIGHT_TAILS / NEAR_SYMMETRIC / INSUFFICIENT_DATA
    pub note: String,
}

/// KYLELAM — Kyle's daily price-impact λ (regression |Δp| on volume).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KylelamSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub kyle_lambda: f64,      // cov(|Δp|, V) / var(V)
    pub mean_abs_dp: f64,      // mean |Δp| ($ price change)
    pub mean_volume: f64,      // mean V (shares)
    pub correlation: f64,      // ρ(|Δp|, V)
    pub r_squared: f64,        // ρ²
    pub kylelam_label: String, // HIGH_IMPACT / MODERATE_IMPACT / LOW_IMPACT / NO_SIGNAL / INSUFFICIENT_DATA
    pub note: String,
}

/// PEAKOVER — Peaks-Over-Threshold (EVT/GPD foundation).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PeakoverSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub threshold_p95: f64,     // P95 of |returns|
    pub threshold_p99: f64,     // P99 of |returns|
    pub count_p95: usize,       // #|r| > P95
    pub count_p99: usize,       // #|r| > P99
    pub mean_excess_p95: f64,   // mean(|r|−P95 | |r|>P95)
    pub mean_excess_p99: f64,   // mean(|r|−P99 | |r|>P99)
    pub max_excess_p95: f64,    // max(|r|−P95)
    pub max_excess_p99: f64,    // max(|r|−P99)
    pub peakover_label: String, // EXTREME_TAIL / HEAVY_TAIL / MODERATE_TAIL / LIGHT_TAIL / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 37 ──────────────────────────────────────────────────────

/// HIGUCHI — Higuchi fractal dimension (Higuchi 1988).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HiguchiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub k_max: usize,          // max sub-sampling interval
    pub fractal_dim: f64,      // FD (slope of log L(k) vs log(1/k))
    pub r_squared: f64,        // linear-fit R²
    pub log_k_count: usize,    // #points used in regression
    pub higuchi_label: String, // SMOOTH / PERSISTENT / RANDOM / ROUGH / INSUFFICIENT_DATA
    pub note: String,
}

/// PICKANDS — Pickands 1975 tail-index estimator.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PickandsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub k_index: usize,         // order-statistic index used
    pub gamma_hat: f64,         // Pickands γ̂ = (1/ln2)·ln((x_k − x_2k)/(x_2k − x_4k))
    pub tail_index: f64,        // 1/γ̂ (Fréchet α, when γ̂ > 0)
    pub x_k: f64,               // k-th largest |r|
    pub x_2k: f64,              // 2k-th largest
    pub x_4k: f64,              // 4k-th largest
    pub pickands_label: String, // FRECHET_HEAVY / FRECHET_MODERATE / GUMBEL_EXPONENTIAL / WEIBULL_BOUNDED / INSUFFICIENT_DATA
    pub note: String,
}

/// KAPPA3 — Kaplan-Knowles 2004 Kappa-3 ratio (third-order LPM).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Kappa3Snapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mar: f64,             // Minimum Acceptable Return (0 for simplicity)
    pub excess_mean: f64,     // μ − MAR (annualised)
    pub lpm3: f64,            // third lower partial moment E[max(MAR−r,0)³]
    pub lpm3_root: f64,       // LPM3^(1/3)
    pub kappa3: f64,          // (μ−MAR) / LPM3^(1/3)
    pub sortino_compare: f64, // (μ−MAR) / LPM2^(1/2) for reference
    pub kappa3_label: String, // STRONG / POSITIVE / NEUTRAL / NEGATIVE / INSUFFICIENT_DATA
    pub note: String,
}

/// LYAPUNOV — Rosenstein et al. 1993 largest Lyapunov exponent.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LyapunovSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize,       // m
    pub time_delay: usize,      // τ (=1 for daily returns)
    pub lambda_max: f64,        // largest Lyapunov exponent (per bar)
    pub r_squared: f64,         // fit quality of ln d(i) vs i
    pub steps_used: usize,      // number of i-steps in the regression
    pub lyapunov_label: String, // CHAOTIC / WEAKLY_CHAOTIC / PERIODIC / STABLE / INSUFFICIENT_DATA
    pub note: String,
}

/// RANKAC — Spearman rank autocorrelation at lags 1, 5, 10.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RankacSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rho_lag1: f64,        // Spearman ρ at lag 1
    pub rho_lag5: f64,        // Spearman ρ at lag 5
    pub rho_lag10: f64,       // Spearman ρ at lag 10
    pub mean_abs_rho: f64,    // mean |ρ| across the 3 lags
    pub max_abs_rho: f64,     // max |ρ|
    pub rankac_label: String, // STRONG_DEPENDENCE / MODERATE_DEPENDENCE / WEAK_DEPENDENCE / INDEPENDENT / INSUFFICIENT_DATA
    pub note: String,
}

/// BNSJUMP — Barndorff-Nielsen & Shephard 2006 jump-detection Z-statistic.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BnsjumpSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub realized_variance: f64, // RV = Σ r_i²
    pub bipower_variance: f64,  // BV = (π/2) · Σ |r_i|·|r_{i-1}|
    pub jump_ratio: f64,        // (RV − BV) / RV  (zero if pure-diffusion)
    pub jump_z_stat: f64,       // (RV − BV) / sqrt(θ · Σ r_i⁴)  (standardised)
    pub p_value: f64,           // 1 − Φ(|z|) (approx)
    pub bnsjump_label: String, // STRONG_JUMP / MODERATE_JUMP / WEAK_JUMP / NO_JUMP / INSUFFICIENT_DATA
    pub note: String,
}

/// PPROOT — Phillips-Perron 1988 nonparametric unit-root test.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PprootSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rho_hat: f64,          // OLS estimate of ρ in y_t = ρ·y_{t-1} + ε
    pub t_rho: f64,            // raw t-statistic for ρ = 1
    pub z_rho: f64,            // Phillips-Perron Z(ρ) statistic (newey-west corrected)
    pub z_t: f64,              // Phillips-Perron Z(t) statistic
    pub lag_truncation: usize, // bandwidth q for the long-run variance
    pub pproot_label: String, // STATIONARY_STRONG / STATIONARY_WEAK / BORDERLINE / UNIT_ROOT / INSUFFICIENT_DATA
    pub note: String,
}

/// MFDFA — Multifractal Detrended Fluctuation Analysis at q ∈ {-2, 0, +2}.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MfdfaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub h_q_neg2: f64,       // generalised Hurst exponent at q = −2
    pub h_q_zero: f64,       // generalised Hurst exponent at q = 0 (median-walk)
    pub h_q_pos2: f64,       // generalised Hurst exponent at q = +2
    pub delta_h: f64,        // h(−2) − h(+2), width of the multifractal spectrum
    pub scales_used: usize,  // number of scales included in the fit
    pub mfdfa_label: String, // STRONG_MULTIFRACTAL / MODERATE_MULTIFRACTAL / WEAK_MULTIFRACTAL / MONOFRACTAL / INSUFFICIENT_DATA
    pub note: String,
}

/// HILLKS — Kolmogorov-Smirnov goodness-of-fit for the Hill-tail Pareto model.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HillksSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub k_order: usize,        // order-stat cutoff used by the Hill estimator
    pub alpha_hat: f64,        // Hill tail index α̂
    pub ks_statistic: f64,     // supremum |F_n − F_pareto| over the tail sample
    pub ks_critical_5pct: f64, // 1.36 / sqrt(k) conventional 5% critical value
    pub hillks_label: String,  // GOOD_FIT / ACCEPTABLE_FIT / POOR_FIT / REJECT / INSUFFICIENT_DATA
    pub note: String,
}

/// TSI — Blau 1991 True Strength Index (double-smoothed momentum oscillator).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TsiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_long: usize,       // long EMA period (default 25)
    pub ema_short: usize,      // short EMA period (default 13)
    pub tsi_value: f64,        // 100 × EMA_short(EMA_long(ΔP)) / EMA_short(EMA_long(|ΔP|))
    pub signal_value: f64,     // EMA_short(tsi_value)
    pub tsi_minus_signal: f64, // tsi − signal (momentum-of-momentum)
    pub tsi_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// GARCH11 — GARCH(1,1) conditional-volatility parameter fit.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Garch11Snapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub omega: f64,             // baseline variance ω
    pub alpha: f64,             // α (ARCH, shock weight)
    pub beta: f64,              // β (GARCH, persistence weight)
    pub persistence: f64,       // α + β
    pub unconditional_var: f64, // ω / (1 − α − β)
    pub half_life_bars: f64,    // ln(0.5) / ln(α+β)  (undefined if α+β ≥ 1)
    pub log_likelihood: f64,    // fitted log-likelihood
    pub garch11_label: String, // HIGH_PERSISTENCE / MODERATE_PERSISTENCE / LOW_PERSISTENCE / NEAR_INTEGRATED / INSUFFICIENT_DATA
    pub note: String,
}

/// SADF — Phillips-Wu-Yu 2011 Sup-ADF explosive-root / bubble test.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SadfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub min_window: usize,      // r0 (minimum window size)
    pub adf_full: f64,          // standard ADF on the full sample
    pub sadf_stat: f64,         // sup over expanding windows of ADF-t
    pub sadf_argmax_end: usize, // end index of the argmax window
    pub critical_95: f64,       // approx 95% critical value for SADF at this n
    pub reject_null: bool,      // true if sadf_stat > critical_95
    pub sadf_label: String, // EXPLOSIVE_CONFIRMED / EXPLOSIVE_LIKELY / BORDERLINE / STABLE / INSUFFICIENT_DATA
    pub note: String,
}

/// CORDIM — Grassberger-Procaccia 1983 correlation dimension D2.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CordimSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize,     // m (embedding dim, default 3)
    pub radii_count: usize,   // number of ε values used in the fit
    pub d2: f64,              // correlation dimension D2
    pub r_squared: f64,       // fit quality of log C(ε) vs log ε
    pub cordim_label: String, // LOW_DIM / MODERATE_DIM / HIGH_DIM / STOCHASTIC / INSUFFICIENT_DATA
    pub note: String,
}

/// SKSPEC — Rolling-window skewness spectrum / stability.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SkspecSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub window_size: usize, // rolling window length (default 30)
    pub mean_skew: f64,     // mean of rolling skew values
    pub std_skew: f64,      // std of rolling skew values (skew stability)
    pub min_skew: f64,
    pub max_skew: f64,
    pub range_skew: f64,      // max − min
    pub skspec_label: String, // STABLE_POSITIVE / STABLE_NEGATIVE / DRIFTING / UNSTABLE / INSUFFICIENT_DATA
    pub note: String,
}

/// AUTOMI — Lag-1 auto-mutual-information (information-theoretic ACF).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AutomiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub num_bins: usize,      // k bins per marginal (default 8)
    pub mi_lag1: f64,         // I(X_t; X_{t-1})  (bits)
    pub mi_lag5: f64,         // I(X_t; X_{t-5})
    pub mi_lag10: f64,        // I(X_t; X_{t-10})
    pub h_marginal: f64,      // H(X) marginal entropy (bits) — baseline
    pub normalized_mi1: f64,  // MI(1) / H(X)  (0..1 fraction of marginal info shared)
    pub automi_label: String, // STRONG / MODERATE / WEAK / INDEPENDENT / INSUFFICIENT_DATA
    pub note: String,
}

/// DURBINWATSON — Durbin-Watson d statistic for AR(1) autocorrelation on returns.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DurbinWatsonSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub dw_stat: f64,      // d ∈ [0, 4], ~2 ⇒ no autocorr
    pub rho_estimate: f64, // implied ρ ≈ 1 − d/2
    pub dw_label: String, // STRONG_POS / WEAK_POS / NO_AUTOCORR / WEAK_NEG / STRONG_NEG / INSUFFICIENT_DATA
    pub note: String,
}

/// BDSTEST — Brock-Dechert-Scheinkman test for iid at embedding dim m=2.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BdsTestSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub embed_dim: usize,       // m (default 2)
    pub epsilon_mult: f64,      // ε = epsilon_mult × σ (default 0.7)
    pub bds_stat: f64,          // standardized test statistic (asymptotically N(0,1))
    pub p_value_two_sided: f64, // 2 × Φ(−|z|)
    pub reject_null: bool,      // p < 0.05
    pub bds_label: String, // IID_CONFIRMED / WEAK_DEPENDENCE / STRONG_DEPENDENCE / INSUFFICIENT_DATA
    pub note: String,
}

/// BREUSCHPAGAN — Breusch-Pagan LM test for heteroskedasticity.
/// Aux regression: squared residual on a simple trend regressor (bar index).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BreuschPaganSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub lm_stat: f64,      // n × R² from aux regression
    pub r_squared: f64,    // R² of aux regression
    pub df: usize,         // degrees of freedom (number of regressors, default 1)
    pub critical_95: f64,  // χ²(df) 95% critical value
    pub reject_null: bool, // true if lm_stat > critical_95
    pub bp_label: String,  // HOMOSKEDASTIC / MILD_HETERO / STRONG_HETERO / INSUFFICIENT_DATA
    pub note: String,
}

/// TURNPTS — Bartels turning-points randomness test on returns.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TurnPtsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub observed_turnpts: usize, // count of local minima + maxima
    pub expected_turnpts: f64,   // 2(n − 2) / 3 under iid
    pub variance_turnpts: f64,   // (16n − 29) / 90
    pub z_stat: f64,             // (obs − exp) / sqrt(var)
    pub p_value_two_sided: f64,
    pub reject_null: bool,
    pub turnpts_label: String, // RANDOM_IID / OVER_TURNING / UNDER_TURNING / INSUFFICIENT_DATA
    pub note: String,
}

/// PERIODOGRAM — discrete Fourier periodogram peak / spectral dominance.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PeriodogramSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub n_freqs: usize,            // number of positive Fourier frequencies tested
    pub dominant_freq: f64,        // cycle frequency (cycles per bar, 0..0.5)
    pub dominant_period_bars: f64, // 1 / dominant_freq
    pub dominant_power: f64,       // spectral power at the peak
    pub total_power: f64,
    pub dominant_power_ratio: f64, // dominant_power / total_power
    pub periodogram_label: String, // STRONG_CYCLE / MODERATE_CYCLE / WEAK_CYCLE / NO_CYCLE / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 41 surfaces ─────────────────────────────────────────────

/// MCLEODLI — McLeod-Li test (Ljung-Box on squared returns for ARCH effects).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct McLeodLiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub lag_h: usize,           // portmanteau window on squared returns
    pub q_stat: f64,            // n(n+2) Σ ρ̂²(k)/(n-k) on squared returns
    pub df: usize,              // lag_h
    pub critical_95: f64,       // χ²(df) 95% critical value
    pub p_value: f64,           // Pr(χ²_df > q_stat)
    pub reject_null: bool,      // true if q_stat > critical_95
    pub mcleodli_label: String, // NO_ARCH / MILD_ARCH / STRONG_ARCH / INSUFFICIENT_DATA
    pub note: String,
}

/// OUFIT — Ornstein-Uhlenbeck mean-reversion fit on log-price.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct OuFitSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub theta: f64,          // mean-reversion speed (per bar)
    pub mu: f64,             // long-run mean log-price
    pub sigma: f64,          // diffusion scale (residual sd of discrete fit)
    pub half_life_bars: f64, // ln(2) / θ; +∞ if θ ≤ 0
    pub residual_sd: f64,    // sd of (x_{t+1} − â − b̂·x_t) residuals
    pub r_squared: f64,      // R² of the AR(1) fit on log-price
    pub oufit_label: String, // TRENDING / SLOW_REVERT / MODERATE_REVERT / FAST_REVERT / INSUFFICIENT_DATA
    pub note: String,
}

/// GPH — Geweke-Porter-Hudak log-periodogram long-memory d estimator.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GphSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub m_freqs: usize,  // truncation m = floor(n^0.5)
    pub d_estimate: f64, // fractional integration order
    pub d_stderr: f64,   // √(π²/24m)
    pub t_stat: f64,     // d / stderr (H0: d=0)
    pub p_value_two_sided: f64,
    pub gph_label: String, // ANTIPERSISTENT / SHORT_MEMORY / LONG_MEMORY / NONSTATIONARY / INSUFFICIENT_DATA
    pub note: String,
}

/// BURGSPEC — Burg maximum-entropy AR-based spectral estimator.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BurgSpecSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ar_order: usize,           // p = min(20, n/4)
    pub dominant_freq: f64,        // cycles per bar at AR-spectrum peak
    pub dominant_period_bars: f64, // 1 / dominant_freq
    pub peak_power: f64,           // AR spectrum at peak
    pub mean_power: f64,           // mean AR-spectrum density over grid
    pub peak_to_mean_ratio: f64,   // peak_power / mean_power
    pub burgspec_label: String, // NO_AR_CYCLE / WEAK_AR_CYCLE / MODERATE_AR_CYCLE / STRONG_AR_CYCLE / INSUFFICIENT_DATA
    pub note: String,
}

/// KENDALLTAU — Kendall's tau lag-1 rank autocorrelation.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KendallTauSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pair_count: usize, // n·(n-1)/2
    pub concordant: usize,
    pub discordant: usize,
    pub tau: f64,    // (C − D) / [n(n−1)/2]
    pub z_stat: f64, // τ / √(2(2n+5)/(9n(n−1)))
    pub p_value_two_sided: f64,
    pub kendalltau_label: String, // STRONG_POS / WEAK_POS / NO_RANK_AUTO / WEAK_NEG / STRONG_NEG / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 42 surfaces ─────────────────────────────────────────────

/// SQUEEZE — composite short-squeeze outlier score per symbol.
/// Fuses five orthogonal axes: short-float %, days-to-cover, 20d momentum,
/// relative volume, and IV-rank. Each axis is normalised to 0..100 and the
/// composite is the weighted mean.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SqueezeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub short_percent_of_float: f64, // raw % of float short
    pub days_to_cover: f64,          // raw days-to-cover (short / 20d vol)
    pub momentum_20d_pct: f64,       // (close_t / close_{t-20}) − 1, in %
    pub relvol_20d: f64,             // current volume / 20d avg
    pub iv_rank: f64,                // 0..100 from IvolSnapshot
    pub short_float_score: f64,      // 0..100 contribution
    pub days_to_cover_score: f64,    // 0..100 contribution
    pub momentum_score: f64,         // 0..100 contribution
    pub relvol_score: f64,           // 0..100 contribution
    pub iv_rank_score: f64,          // 0..100 contribution
    pub composite_score: f64,        // 0..100 weighted mean
    pub inputs_present: usize,       // how many of the 5 axes had data (0..5)
    pub squeeze_label: String, // NO_SQUEEZE / WATCH / ELEVATED / STRONG / EXTREME / INSUFFICIENT_DATA
    pub note: String,
}

/// SQUEEZERANK — cross-symbol percentile rank of SQUEEZE composite scores.
/// Populated by a table-scan across all symbols with a SQUEEZE row.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SqueezeRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub composite_score: f64,      // mirror of SQUEEZE.composite_score
    pub peer_count: usize,         // symbols scanned
    pub rank: usize,               // 1 = highest composite
    pub percentile: f64,           // 0..100
    pub squeezerank_label: String, // TOP_1PCT / TOP_5PCT / TOP_10PCT / ABOVE_MEDIAN / BELOW_MEDIAN / INSUFFICIENT_DATA
    pub note: String,
}

/// BBSQUEEZE — Bollinger-Band squeeze detector.
/// Uses 20-bar SMA ±2σ; BB-width = (upper-lower)/mid. A "squeeze" is when
/// the current BB-width is in the low tail of its 120-bar history.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BbsqueezeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,         // 20
    pub bb_width_current: f64, // (upper - lower) / mid at last bar
    pub bb_width_min_120: f64,
    pub bb_width_max_120: f64,
    pub bb_width_percentile: f64, // 0..100 current rank in 120-bar history
    pub upper_band: f64,
    pub lower_band: f64,
    pub mid_band: f64,
    pub last_close: f64,
    pub bbsqueeze_label: String, // TIGHT_SQUEEZE / MODERATE_SQUEEZE / NORMAL / EXPANSION / INSUFFICIENT_DATA
    pub note: String,
}

/// DONCHIAN — Donchian-channel breakout detector (20-bar default).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DonchianSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,      // 20
    pub upper_channel: f64, // max(high) over period
    pub lower_channel: f64, // min(low) over period
    pub mid_channel: f64,   // (upper + lower)/2
    pub last_close: f64,
    pub channel_position_pct: f64, // 0..100, (close-lower)/(upper-lower)
    pub breakout_upper: bool,      // close ≥ prior upper
    pub breakout_lower: bool,      // close ≤ prior lower
    pub donchian_label: String, // BREAKOUT_UP / APPROACH_UP / NEUTRAL / APPROACH_DOWN / BREAKOUT_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// KAMA — Kaufman Adaptive Moving Average efficiency ratio.
/// Efficiency Ratio = |close_t - close_{t-n}| / Σ|close_i - close_{i-1}|.
/// High ER = trending; low ER = choppy.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KamaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,         // 10
    pub efficiency_ratio: f64, // 0..1
    pub kama_value: f64,       // adaptive MA at last bar
    pub last_close: f64,
    pub kama_slope_pct: f64, // (kama_t / kama_{t-5} - 1) × 100
    pub kama_label: String, // STRONG_TREND / MODERATE_TREND / WEAK_TREND / CHOPPY / INSUFFICIENT_DATA
    pub note: String,
}

/// ICHIMOKU — Ichimoku Kinko Hyo five-line cloud system.
/// Tenkan 9, Kijun 26, Senkou A = (Tenkan+Kijun)/2 plotted +26, Senkou B = 52-bar
/// midpoint plotted +26, Chikou = close plotted −26. All midpoints use (H+L)/2.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IchimokuSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub tenkan_sen: f64,    // 9-bar midpoint
    pub kijun_sen: f64,     // 26-bar midpoint
    pub senkou_span_a: f64, // (tenkan+kijun)/2 at t (projected +26)
    pub senkou_span_b: f64, // 52-bar midpoint at t (projected +26)
    pub chikou_span: f64,   // close plotted back −26
    pub cloud_top: f64,     // max(senkou_a, senkou_b)
    pub cloud_bottom: f64,  // min(senkou_a, senkou_b)
    pub last_close: f64,
    pub close_vs_cloud_pct: f64, // (close - cloud_mid) / cloud_mid × 100
    pub ichimoku_label: String, // STRONG_BULL / BULL / IN_CLOUD / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// SUPERTREND — ATR-based trailing-stop trend indicator.
/// Period 10, multiplier 3. Flips on close crossing prior band.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SupertrendSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,   // 10
    pub multiplier: f64, // 3.0
    pub atr: f64,
    pub upper_band: f64,
    pub lower_band: f64,
    pub supertrend_value: f64, // active band (upper in downtrend, lower in up)
    pub trend_is_up: bool,
    pub last_close: f64,
    pub distance_pct: f64,        // (close - supertrend) / supertrend × 100
    pub bars_in_trend: usize,     // bars since last flip
    pub supertrend_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// KELTNER — Keltner Channels (EMA 20 ± multiplier × ATR 10).
/// Pairs with BBSQUEEZE for the TTM-squeeze (BB inside KC → volatility compression).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KeltnerSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_period: usize, // 20
    pub atr_period: usize, // 10
    pub multiplier: f64,   // 2.0
    pub ema_value: f64,    // midline
    pub atr: f64,
    pub upper_channel: f64,
    pub lower_channel: f64,
    pub last_close: f64,
    pub channel_width: f64,        // upper - lower
    pub width_pct_of_mid: f64,     // (upper - lower) / ema × 100
    pub channel_position_pct: f64, // (close - lower) / (upper - lower) × 100
    pub ttm_squeeze_on: bool,      // true when BB fully inside KC (computed here using BB 20/2σ)
    pub keltner_label: String, // BREAKOUT_UP / NEAR_UPPER / IN_CHANNEL / NEAR_LOWER / BREAKOUT_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// FISHER — John Ehlers' Fisher Transform of normalised price.
/// Normalises close to [-1, 1] window, then applies 0.5·ln((1+x)/(1-x)).
/// Output distribution is approximately Gaussian; sharp turning points.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FisherSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,         // 10
    pub fisher_value: f64,     // latest transform output
    pub fisher_signal: f64,    // prior bar's fisher_value (the "trigger")
    pub extreme_2_cross: bool, // crossed ±2 in last 3 bars (mean-reversion flag)
    pub peak_abs_10: f64,      // max |fisher| over last 10 bars
    pub last_close: f64,
    pub fisher_label: String, // STRONG_POS / POS / NEUTRAL / NEG / STRONG_NEG / INSUFFICIENT_DATA
    pub note: String,
}

/// AROON — Aroon Up / Aroon Down / Aroon Oscillator over 25 bars.
/// Aroon Up = 100 × (period − bars_since_high) / period
/// Aroon Down = 100 × (period − bars_since_low) / period
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AroonSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,         // 25
    pub aroon_up: f64,         // 0..100
    pub aroon_down: f64,       // 0..100
    pub aroon_oscillator: f64, // up − down, −100..100
    pub bars_since_high: usize,
    pub bars_since_low: usize,
    pub last_close: f64,
    pub aroon_label: String, // STRONG_UP / WEAK_UP / CONSOLIDATION / WEAK_DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// ADX — Wilder's Average Directional Index (period 14).
/// Reports +DI, -DI, ADX and directional-movement smoothed averages.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AdxSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub plus_di: f64,
    pub minus_di: f64,
    pub adx: f64,
    pub dx: f64, // instantaneous |+DI − −DI|/(+DI + −DI) × 100
    pub atr: f64,
    pub last_close: f64,
    pub adx_label: String, // STRONG_TREND adx≥40 / TREND ≥25 / WEAK_TREND ≥15 / NO_TREND / INSUFFICIENT_DATA
    pub note: String,
}

/// CCI — Lambert's Commodity Channel Index (period 20).
/// CCI = (TP − SMA(TP)) / (0.015 × MD) where TP=(H+L+C)/3, MD is mean absolute deviation.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CciSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 20
    pub typical_price: f64,
    pub tp_sma: f64,
    pub mean_abs_dev: f64,
    pub cci_value: f64,
    pub last_close: f64,
    pub cci_label: String, // OVERBOUGHT >100 / BULL >0 / NEUTRAL / BEAR <0 / OVERSOLD <−100 / INSUFFICIENT_DATA
    pub note: String,
}

/// CMF — Chaikin Money Flow (period 20).
/// Σ(MFV)/Σ(volume) over window, where MFV = ((close−low) − (high−close))/(high−low) × volume.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CmfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,  // 20
    pub cmf_value: f64, // −1..+1
    pub money_flow_volume_sum: f64,
    pub volume_sum: f64,
    pub last_close: f64,
    pub cmf_label: String, // STRONG_ACCUM >0.25 / ACCUM >0.05 / NEUTRAL / DIST <−0.05 / STRONG_DIST <−0.25 / INSUFFICIENT_DATA
    pub note: String,
}

/// MFI — Quong & Soudack's Money Flow Index (period 14).
/// Volume-weighted RSI: uses typical price × volume as "money flow".
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MfiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,  // 14
    pub mfi_value: f64, // 0..100
    pub positive_mf_sum: f64,
    pub negative_mf_sum: f64,
    pub money_flow_ratio: f64,
    pub last_close: f64,
    pub mfi_label: String, // OVERBOUGHT >80 / BULL >50 / NEUTRAL / BEAR <50 / OVERSOLD <20 / INSUFFICIENT_DATA
    pub note: String,
}

/// PSAR — Wilder's Parabolic Stop-And-Reverse.
/// Initial AF 0.02, increment 0.02, cap 0.20. Flips when price crosses SAR.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PsarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub af_start: f64,            // 0.02
    pub af_step: f64,             // 0.02
    pub af_max: f64,              // 0.20
    pub sar_value: f64,           // latest SAR
    pub extreme_point: f64,       // EP (highest high in long trend, lowest low in short)
    pub acceleration_factor: f64, // current AF
    pub trend_is_up: bool,
    pub bars_in_trend: usize,
    pub distance_pct: f64, // (close - sar) / sar × 100
    pub last_close: f64,
    pub psar_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// VORTEX — Botes & Siepman (2009) directional-movement alternative.
/// VI+ = Σ|H_t − L_{t−1}| / ΣTR, VI− = Σ|L_t − H_{t−1}| / ΣTR over period=14.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VortexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub vi_plus: f64,
    pub vi_minus: f64,
    pub vi_diff: f64, // VI+ − VI−
    pub sum_tr: f64,
    pub sum_vm_plus: f64,
    pub sum_vm_minus: f64,
    pub last_close: f64,
    pub vortex_label: String, // BULL_CROSS / BULL / NEUTRAL / BEAR / BEAR_CROSS / INSUFFICIENT_DATA
    pub note: String,
}

/// CHOP — Bill Dreiss Choppiness Index (period=14).
/// CI = 100·log10(ΣTR / (maxH − minL)) / log10(N). Values > 61.8 = choppy, < 38.2 = trending.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ChopSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,   // 14
    pub chop_value: f64, // 0..100
    pub sum_tr: f64,
    pub range_high: f64, // max high over period
    pub range_low: f64,  // min low over period
    pub range_span: f64, // range_high - range_low
    pub last_close: f64,
    pub chop_label: String, // CHOP >61.8 / RANGING >50 / NEUTRAL / TRANSITIONAL <50 / TRENDING <38.2 / INSUFFICIENT_DATA
    pub note: String,
}

/// OBV — Granville (1963) On-Balance Volume cumulative + 20-bar slope.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ObvSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub slope_window: usize, // 20
    pub obv_value: f64,      // latest cumulative
    pub obv_slope: f64,      // linear-regression slope of last 20 OBV values
    pub obv_change_pct: f64, // (obv[N-1] - obv[N-20]) / |obv[N-20]| × 100 (or 0 if divisor≈0)
    pub obv_min_20: f64,
    pub obv_max_20: f64,
    pub last_close: f64,
    pub obv_label: String, // STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TRIX — Jack Hutson (1980s) triple-EMA momentum oscillator.
/// EMA3 = EMA(EMA(EMA(close, N), N), N); TRIX = 100·(EMA3_t/EMA3_{t−1} − 1); signal = EMA(TRIX, 9).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TrixSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,        // 15
    pub signal_period: usize, // 9
    pub trix_value: f64,      // % change
    pub signal_value: f64,    // EMA(TRIX, 9)
    pub histogram: f64,       // trix − signal
    pub ema3_value: f64,      // final triple-smoothed EMA level
    pub last_close: f64,
    pub trix_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// HMA — Alan Hull (2005) weighted-moving-average combo: HMA = WMA(2·WMA(n/2) − WMA(n), √n).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HmaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,         // 20
    pub half_period: usize,    // 10
    pub sqrt_period: usize,    // 4 (floor(sqrt(20)))
    pub hma_value: f64,        // latest HMA
    pub hma_slope_pct: f64,    // (hma[N-1] - hma[N-6]) / hma[N-6] × 100
    pub hma_vs_close_pct: f64, // (close - hma) / hma × 100
    pub last_close: f64,
    pub hma_label: String, // STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// PPO — Gerald Appel Percentage Price Oscillator: 100·(EMA_fast − EMA_slow)/EMA_slow.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PpoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize,   // 12
    pub slow_period: usize,   // 26
    pub signal_period: usize, // 9
    pub ema_fast: f64,
    pub ema_slow: f64,
    pub ppo_value: f64,    // 100·(ema_fast − ema_slow)/ema_slow
    pub signal_value: f64, // EMA(ppo, 9)
    pub histogram: f64,    // ppo − signal
    pub last_close: f64,
    pub ppo_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// DPO — Detrended Price Oscillator: close − SMA(close, N) shifted back (N/2 + 1) bars.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DpoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,  // 20
    pub shift: usize,   // N/2 + 1 = 11
    pub sma_value: f64, // SMA at shifted index
    pub dpo_value: f64, // close[t - shift] − sma[t]
    pub dpo_pct: f64,   // dpo / sma × 100
    pub last_close: f64,
    pub dpo_label: String, // PEAK_HIGH / BULL / NEUTRAL / BEAR / PEAK_LOW / INSUFFICIENT_DATA
    pub note: String,
}

/// KST — Martin Pring Know Sure Thing: weighted sum of four ROCs smoothed by SMA.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KstSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rcma1: f64,        // SMA(ROC(10), 10)
    pub rcma2: f64,        // SMA(ROC(15), 10)
    pub rcma3: f64,        // SMA(ROC(20), 10)
    pub rcma4: f64,        // SMA(ROC(30), 15)
    pub kst_value: f64,    // 1·rcma1 + 2·rcma2 + 3·rcma3 + 4·rcma4
    pub signal_value: f64, // SMA(kst, 9)
    pub histogram: f64,    // kst − signal
    pub last_close: f64,
    pub kst_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// ULTOSC — Larry Williams Ultimate Oscillator: weighted 3-period BP/TR ratio.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct UltoscSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period_short: usize, // 7
    pub period_mid: usize,   // 14
    pub period_long: usize,  // 28
    pub avg_short: f64,      // ΣBP_7 / ΣTR_7
    pub avg_mid: f64,        // ΣBP_14 / ΣTR_14
    pub avg_long: f64,       // ΣBP_28 / ΣTR_28
    pub ultosc_value: f64,   // 100·(4·avg_short + 2·avg_mid + avg_long) / 7
    pub last_close: f64,
    pub ultosc_label: String, // OVERBOUGHT >70 / BULL >50 / NEUTRAL / BEAR <50 / OVERSOLD <30 / INSUFFICIENT_DATA
    pub note: String,
}

/// WILLR — Larry Williams %R: (highest_high − close) / (highest_high − lowest_low) · −100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WillrSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub highest_high: f64,
    pub lowest_low: f64,
    pub willr_value: f64, // ∈ [−100, 0]
    pub last_close: f64,
    pub willr_label: String, // OVERBOUGHT >-20 / BULL >-50 / NEUTRAL / BEAR <-50 / OVERSOLD <-80 / INSUFFICIENT_DATA
    pub note: String,
}

/// MASS — Donald Dorsey (1992) Mass Index: reversal-detection from H-L range expansion.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MassSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_period: usize, // 9
    pub sum_period: usize, // 25
    pub mass_value: f64,   // Σ(EMA₉(H−L) / EMA₉(EMA₉(H−L))) over 25 bars
    pub single_ratio: f64, // latest per-bar ratio
    pub last_close: f64,
    pub mass_label: String, // REVERSAL_BULGE >27 crossing back <26.5 (sentinel NEAR) / WATCH >25 / NEUTRAL / INSUFFICIENT_DATA
    pub note: String,
}

/// CHAIKOSC — Marc Chaikin Oscillator: MACD (fast-slow EMA) of the Accumulation/Distribution line.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ChaikoscSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize, // 3
    pub slow_period: usize, // 10
    pub ad_last: f64,       // latest A/D line
    pub ema_fast_ad: f64,
    pub ema_slow_ad: f64,
    pub chaikosc_value: f64, // ema_fast_ad − ema_slow_ad
    pub last_close: f64,
    pub chaikosc_label: String, // STRONG_ACCUM / ACCUM / NEUTRAL / DIST / STRONG_DIST / INSUFFICIENT_DATA
    pub note: String,
}

/// KLINGER — Stephen Klinger Volume Oscillator: volume-force EMA spread with signal.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KlingerSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize,   // 34
    pub slow_period: usize,   // 55
    pub signal_period: usize, // 13
    pub ema_fast_vf: f64,
    pub ema_slow_vf: f64,
    pub kvo_value: f64,    // ema_fast_vf − ema_slow_vf
    pub signal_value: f64, // EMA(kvo, 13)
    pub histogram: f64,    // kvo − signal
    pub last_close: f64,
    pub klinger_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// STOCHRSI — Tushar Chande (1994) Stochastic RSI: Stochastic applied to the RSI series.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StochRsiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rsi_period: usize,   // 14
    pub stoch_period: usize, // 14
    pub k_period: usize,     // 3 (%K smoothing)
    pub d_period: usize,     // 3 (%D smoothing)
    pub rsi_value: f64,      // underlying RSI
    pub rsi_min: f64,        // min RSI over stoch_period
    pub rsi_max: f64,        // max RSI over stoch_period
    pub stoch_rsi_raw: f64,  // (RSI − min) / (max − min) · 100
    pub k_value: f64,        // SMA(raw, 3)
    pub d_value: f64,        // SMA(%K, 3)
    pub last_close: f64,
    pub stochrsi_label: String, // OVERBOUGHT >80 / BULL >50 / NEUTRAL / BEAR <50 / OVERSOLD <20 / INSUFFICIENT_DATA
    pub note: String,
}

/// AWESOME — Bill Williams Awesome Oscillator: SMA5(hl2) − SMA34(hl2).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AwesomeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize, // 5
    pub slow_period: usize, // 34
    pub sma_fast: f64,      // SMA(hl2, 5)
    pub sma_slow: f64,      // SMA(hl2, 34)
    pub ao_value: f64,      // fast − slow
    pub ao_prev: f64,       // prior bar AO (for color signal)
    pub ao_color_up: bool,  // true if ao_value > ao_prev
    pub last_close: f64,
    pub awesome_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// EFI — Alexander Elder (1993) Force Index: volume-weighted close change, smoothed by EMA13.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct EfiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_period: usize, // 13
    pub raw_efi: f64,      // volume * (close − prev_close) latest bar
    pub efi_value: f64,    // EMA13 of raw_efi
    pub efi_prev: f64,     // prior bar EFI (zero-cross detection)
    pub last_close: f64,
    pub efi_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// EMV — Richard Arms (1980s) Ease of Movement: distance-moved / box-ratio smoothed by SMA14.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct EmvSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub sma_period: usize, // 14
    pub volume_scale: f64, // 100_000_000 (100 M)
    pub raw_emv: f64,      // distance_moved / box_ratio latest bar
    pub emv_value: f64,    // SMA14 of raw_emv
    pub last_close: f64,
    pub emv_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// NVI — Paul Dysart / Norman Fosback Negative Volume Index: updates only on down-volume days.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NviSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub signal_period: usize, // 255 (EMA of NVI line)
    pub nvi_value: f64,       // latest NVI (starts at 1000)
    pub signal_value: f64,    // EMA255 of NVI
    pub last_close: f64,
    pub nvi_label: String, // BULL (nvi > signal) / NEUTRAL / BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// PVI — Paul Dysart / Norman Fosback Positive Volume Index: updates only on up-volume days.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PviSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub signal_period: usize, // 255
    pub pvi_value: f64,       // latest PVI (starts at 1000)
    pub signal_value: f64,    // EMA255 of PVI
    pub last_close: f64,
    pub pvi_label: String, // BULL / NEUTRAL / BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// COPPOCK — E.S.C. Coppock (1962) Coppock Curve: WMA10(ROC14 + ROC11) long-term momentum.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CoppockSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub roc_fast: usize,    // 11
    pub roc_slow: usize,    // 14
    pub wma_period: usize,  // 10
    pub coppock_value: f64, // current reading
    pub coppock_prev: f64,  // prior bar
    pub last_close: f64,
    pub coppock_label: String, // BUY_CROSS (prev<0, now>0) / BULL / NEUTRAL / BEAR / SELL_CROSS / INSUFFICIENT_DATA
    pub note: String,
}

/// CMO — Tushar Chande (1994) Momentum Oscillator: raw gain/loss spread on [-100, +100].
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CmoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,  // 9
    pub sum_up: f64,    // Σ positive close changes over period
    pub sum_dn: f64,    // Σ |negative close changes| over period
    pub cmo_value: f64, // 100 · (sum_up − sum_dn) / (sum_up + sum_dn)
    pub last_close: f64,
    pub cmo_label: String, // OVERBOUGHT >50 / BULL >0 / NEUTRAL / BEAR <0 / OVERSOLD <−50 / INSUFFICIENT_DATA
    pub note: String,
}

/// QSTICK — Tushar Chande (1995) Q-Stick: SMA of candle body (close − open).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct QstickSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,     // 14
    pub qstick_value: f64, // SMA(close − open, 14)
    pub qstick_prev: f64,  // prior bar
    pub last_close: f64,
    pub qstick_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// DISPARITY — Steve Nison popularised (Japanese origin) Disparity Index: % deviation from SMA.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DisparitySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub sma_value: f64,
    pub disparity_value: f64, // (close / SMA − 1) · 100
    pub last_close: f64,
    pub disparity_label: String, // STRONG_BULL >3 / BULL >0 / NEUTRAL / BEAR <0 / STRONG_BEAR <−3 / INSUFFICIENT_DATA
    pub note: String,
}

/// BOP — Igor Livshin Balance of Power: (close − open) / (high − low), smoothed by SMA.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BopSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,  // 14
    pub raw_bop: f64,   // latest bar raw BOP
    pub bop_value: f64, // SMA14 of raw BOP
    pub last_close: f64,
    pub bop_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// SCHAFF — Doug Schaff (2008) Schaff Trend Cycle: stochastic-of-MACD, double-smoothed.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SchaffSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_fast: usize, // 23
    pub ema_slow: usize, // 50
    pub cycle: usize,    // 10
    pub stc_value: f64,  // current STC in [0, 100]
    pub stc_prev: f64,   // prior bar (for direction)
    pub last_close: f64,
    pub schaff_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// STOCH — Lane's classic Stochastic Oscillator (%K fast + %D slow).
/// Distinct from STOCHRSI which applies the stochastic to RSI instead of price.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StochSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub k_period: usize,  // 14
    pub d_period: usize,  // 3
    pub smoothing: usize, // 3 (slow %K smoothing)
    pub percent_k: f64,   // current slow %K in [0, 100]
    pub percent_d: f64,   // current %D (SMA of %K)
    pub last_close: f64,
    pub stoch_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// MACD — Gerald Appel (1979) Moving Average Convergence Divergence.
/// 12/26 EMAs, 9-period signal EMA, histogram = MACD - signal.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MacdSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize,   // 12
    pub slow_period: usize,   // 26
    pub signal_period: usize, // 9
    pub macd_value: f64,      // fast_ema - slow_ema
    pub signal_value: f64,    // 9-EMA of macd_value
    pub histogram: f64,       // macd_value - signal_value
    pub histogram_prev: f64,  // previous bar's histogram (for direction)
    pub last_close: f64,
    pub macd_label: String, // BULL_CROSS / BULL / NEUTRAL / BEAR / BEAR_CROSS / INSUFFICIENT_DATA
    pub note: String,
}

/// VWAP — Volume Weighted Average Price computed over a rolling window.
/// VWAP = Σ(typical_price × volume) / Σ(volume) where typical_price = (H+L+C)/3.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VwapSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub window: usize,   // 20
    pub vwap_value: f64, // current VWAP over `window` bars
    pub last_close: f64,
    pub deviation_pct: f64, // (last_close - vwap) / vwap × 100
    pub vwap_label: String, // STRONG_ABOVE / ABOVE / AT / BELOW / STRONG_BELOW / INSUFFICIENT_DATA
    pub note: String,
}

/// MCGD — John McGinley (1997) McGinley Dynamic adaptive moving average.
/// MD[i] = MD[i-1] + (P - MD[i-1]) / (N × (P/MD[i-1])^4). Designed to resist whipsaws.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct McgdSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,   // 14
    pub mcgd_value: f64, // current McGinley Dynamic
    pub mcgd_prev: f64,  // prior bar (for slope direction)
    pub last_close: f64,
    pub deviation_pct: f64, // (last_close - mcgd) / mcgd × 100
    pub mcgd_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// RWI — Michael Poulos (1991) Random Walk Index.
/// Measures how far price has moved vs a random-walk expectation. Ratios > 1
/// mean the move is larger than what random noise would produce at that horizon.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RwiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14 (max lookback used for ratios)
    pub rwi_high: f64, // max ratio over 2..=length
    pub rwi_low: f64,  // max ratio over 2..=length
    pub last_close: f64,
    pub rwi_label: String, // TRENDING_UP / TRENDING_DOWN / RANGE_BOUND / INSUFFICIENT_DATA
    pub note: String,
}

/// DEMA — Patrick Mulloy (1994) Double Exponential MA.
/// DEMA = 2·EMA(N) − EMA(EMA(N)). Reduces the lag of a plain EMA.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DemaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub dema_value: f64,
    pub dema_prev: f64,     // prior-bar DEMA (for slope)
    pub deviation_pct: f64, // (last_close - dema) / dema × 100
    pub last_close: f64,
    pub dema_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// TEMA — Patrick Mulloy (1994) Triple Exponential MA.
/// TEMA = 3·EMA(N) − 3·EMA(EMA(N)) + EMA(EMA(EMA(N))). Even less lag than DEMA.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TemaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub tema_value: f64,
    pub tema_prev: f64,
    pub deviation_pct: f64,
    pub last_close: f64,
    pub tema_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// LINREG — OLS linear regression on last N closes (slope + R² + channel bounds).
/// Slope in price/bar units; R² bounded [0, 1] (fit quality); channel = ±1σ of residuals.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LinregSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,  // 20
    pub slope: f64,     // fit line slope (price units per bar)
    pub intercept: f64, // fit line value at bar 0 of window
    pub r_squared: f64, // coefficient of determination [0, 1]
    pub sigma: f64,     // standard error of residuals
    pub last_close: f64,
    pub fit_value: f64,       // regression line at the final bar
    pub channel_upper: f64,   // fit_value + 2σ
    pub channel_lower: f64,   // fit_value − 2σ
    pub linreg_label: String, // STRONG_UP_TREND / UP_TREND / RANGE / DOWN_TREND / STRONG_DOWN_TREND / INSUFFICIENT_DATA
    pub note: String,
}

/// PIVOTS — classic floor-trader daily pivot levels computed from the prior bar.
/// PP = (H + L + C) / 3; R1 = 2·PP − L; S1 = 2·PP − H; R2 = PP + (H − L); S2 = PP − (H − L).
/// Header labels where the current close sits relative to the levels.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PivotsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pp: f64, // central pivot point
    pub r1: f64,
    pub r2: f64,
    pub s1: f64,
    pub s2: f64,
    pub last_close: f64,
    pub prior_high: f64,      // the H used
    pub prior_low: f64,       // the L used
    pub prior_close: f64,     // the C used
    pub pivots_label: String, // ABOVE_R2 / BETWEEN_R1_R2 / BETWEEN_PP_R1 / AT_PP / BETWEEN_S1_PP / BETWEEN_S2_S1 / BELOW_S2 / INSUFFICIENT_DATA
    pub note: String,
}

/// HEIKIN — Heikin Ashi candle sentiment tracker.
/// HA_close = (O+H+L+C)/4; HA_open = (prior_HA_open + prior_HA_close)/2;
/// HA_high = max(H, HA_open, HA_close); HA_low = min(L, HA_open, HA_close).
/// Tracks consecutive same-color count and current body/wick geometry.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HeikinSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ha_open: f64,
    pub ha_high: f64,
    pub ha_low: f64,
    pub ha_close: f64,
    pub body_abs: f64,                 // |HA_close − HA_open|
    pub upper_wick: f64,               // HA_high − max(HA_open, HA_close)
    pub lower_wick: f64,               // min(HA_open, HA_close) − HA_low
    pub consecutive_same_color: usize, // count of bars in current run (inclusive of current)
    pub last_close: f64,
    pub heikin_label: String, // STRONG_BULL_RUN / BULL / DOJI / BEAR / STRONG_BEAR_RUN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 52: ALMA / ZLEMA / ELDERRAY / TSF / RVI ──────────────────

/// ALMA — Arnaud Legoux Moving Average with Gaussian kernel.
/// weights[i] = exp(-((i - m)^2) / (2*s^2)) where m = offset*(N-1), s = N/sigma.
/// Default length 20, offset 0.85, sigma 6.0. First Gaussian-kernel MA in the packet.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AlmaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,
    pub offset: f64,
    pub sigma: f64,
    pub alma_value: f64,
    pub alma_prev: f64,
    pub deviation_pct: f64, // (last_close − alma_value) / alma_value × 100
    pub last_close: f64,
    pub alma_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// ZLEMA — Zero-Lag EMA (Ehlers). De-lag shift of `(N-1)/2` on the input
/// before computing the EMA. length 20 → lag 9.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ZlemaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,
    pub lag_shift: usize,
    pub zlema_value: f64,
    pub zlema_prev: f64,
    pub deviation_pct: f64, // (last_close − zlema_value) / zlema_value × 100
    pub last_close: f64,
    pub zlema_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// ELDERRAY — Dr. Alexander Elder's Bull/Bear Power.
/// Bull = high − EMA13(close). Bear = low − EMA13(close).
/// Dual-channel trend intensity: Bull measures upward force, Bear measures downward force.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ElderRaySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_length: usize, // 13 by default
    pub ema13: f64,
    pub ema13_prev: f64,
    pub bull_power: f64, // high − EMA13
    pub bull_power_prev: f64,
    pub bear_power: f64, // low − EMA13
    pub bear_power_prev: f64,
    pub last_close: f64,
    pub elder_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// TSF — Time Series Forecast. OLS over last N closes, forecast one bar forward.
/// Projects the LINREG slope forward: forecast = slope·N + intercept (using 0..N time indices).
/// Complements LINREG (current-bar fit) with LEADING/LAGGING classification.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TsfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,       // 20
    pub slope: f64,          // per-bar slope
    pub intercept: f64,      // at t=0 of the window
    pub forecast_value: f64, // projected close at t=N (one bar ahead)
    pub last_close: f64,
    pub forecast_deviation_pct: f64, // (forecast_value − last_close) / last_close × 100
    pub r_squared: f64,              // goodness of fit
    pub tsf_label: String, // LEADING_UP / LAGGING_UP / LEADING_DOWN / LAGGING_DOWN / FLAT / INSUFFICIENT_DATA
    pub note: String,
}

/// RVI — Relative Vigor Index (John Ehlers / Markos Katsanos).
/// Numerator[i] = (C−O)[i] + 2·(C−O)[i−1] + 2·(C−O)[i−2] + (C−O)[i−3] (triangular weighting)
/// Denominator[i] = same weighting on (H−L)
/// RVI = SMA(numerator, 10) / SMA(denominator, 10)
/// Signal = (RVI[i] + 2·RVI[i−1] + 2·RVI[i−2] + RVI[i−3]) / 6
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RviSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 10
    pub rvi_value: f64,
    pub rvi_prev: f64,
    pub signal_value: f64,
    pub signal_prev: f64,
    pub last_close: f64,
    pub rvi_label: String, // BULL_CROSS / BEAR_CROSS / BULL / BEAR / NEUTRAL / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 53: TRIMA / T3 / VIDYA / SMI / PVT ───────────────────────

/// TRIMA — Triangular Moving Average. SMA-of-SMA with a (N+1)/2 sub-window
/// produces a triangular-weighted central MA. Distinct from SMA (flat),
/// WMA/HMA (linear), EMA (exponential), ALMA (Gaussian), DEMA/TEMA
/// (algebraic lag reduction). Length 20.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TrimaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub trima_value: f64,
    pub trima_prev: f64,
    pub deviation_pct: f64, // (last_close − trima_value) / trima_value × 100
    pub last_close: f64,
    pub trima_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// T3 — Tim Tillson's 1998 composite MA. Six iterative EMAs with volume
/// factor v default 0.7:
/// e1 = EMA(close, N); e2 = EMA(e1, N); ... ; e6 = EMA(e5, N);
/// c1 = −v³; c2 = 3v² + 3v³; c3 = −6v² − 3v − 3v³; c4 = 1 + 3v + v³ + 3v²;
/// T3 = c1·e6 + c2·e5 + c3·e4 + c4·e3.
/// Generalises DEMA (v=0 recovers EMA; v=1 produces strong lag reduction).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct T3Snapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub v_factor: f64, // 0.7 canonical
    pub t3_value: f64,
    pub t3_prev: f64,
    pub deviation_pct: f64,
    pub last_close: f64,
    pub t3_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// VIDYA — Tushar Chande's 1992 Variable Index Dynamic Average.
/// alpha_t = (2 / (N+1)) · |CMO(9)| / 100.
/// VIDYA_t = alpha_t · price_t + (1 − alpha_t) · VIDYA_{t−1}.
/// alpha scales with momentum: strong trends accelerate the MA, ranges freeze it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VidyaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,     // 20 (EMA base)
    pub cmo_length: usize, // 9
    pub vidya_value: f64,
    pub vidya_prev: f64,
    pub current_alpha: f64, // last-bar effective alpha
    pub cmo_magnitude: f64, // |CMO| at last bar ∈ [0, 100]
    pub deviation_pct: f64,
    pub last_close: f64,
    pub vidya_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// SMI — Stochastic Momentum Index (William Blau 1993).
/// H = max(high, N); L = min(low, N); mid = (H+L)/2.
/// Numerator = double-EMA smoothed (close − mid).
/// Denominator = double-EMA smoothed ((H−L)/2).
/// SMI = 100 · Numerator / Denominator ∈ [−100, 100].
/// Signal = EMA(SMI, short).
/// Distinct from STOCHRSI (stochastic of RSI) and STOCH (raw price stochastic).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SmiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,        // 10 lookback
    pub smooth_length: usize, // 3 double-EMA smoothing
    pub signal_length: usize, // 3 signal EMA
    pub smi_value: f64,
    pub smi_prev: f64,
    pub signal_value: f64,
    pub signal_prev: f64,
    pub last_close: f64,
    pub smi_label: String, // OVERBOUGHT / BULL_CROSS / BULL / NEUTRAL / BEAR / BEAR_CROSS / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// PVT — Price Volume Trend (Dysart/Lowry 1966).
/// PVT_t = PVT_{t−1} + volume_t · (close_t − close_{t−1}) / close_{t−1}.
/// Cumulative volume-weighted running sum of percent price changes.
/// Distinct from OBV (±volume based on close direction): PVT scales the
/// volume attribution by the magnitude of the percent move.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PvtSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pvt_value: f64, // cumulative PVT at last bar
    pub pvt_prev: f64,  // cumulative PVT at previous bar
    pub pvt_ema: f64,   // 20-bar EMA of PVT series
    pub pvt_slope: f64, // PVT[last] − PVT[last−n], n=20
    pub last_close: f64,
    pub pvt_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 54: AC / CHVOL / BBWIDTH / ELDERIMP / RMI ───────────────

/// Bill Williams's Accelerator Oscillator — a second-derivative momentum
/// indicator built as `AC = AO − SMA₅(AO)` where
/// `AO = SMA₅(medprice) − SMA₃₄(medprice)`. Flags acceleration direction
/// relative to the AO trend.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AcSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ac_value: f64, // AO − SMA5(AO)
    pub ac_prev: f64,
    pub ao_value: f64, // current Awesome Oscillator
    pub ao_sma5: f64,  // 5-SMA of AO
    pub last_close: f64,
    pub ac_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Marc Chaikin's Volatility indicator — the rate-of-change of the 10-bar
/// EMA of the high-low range over a 10-bar lookback. Positive readings
/// indicate range expansion; negative readings indicate contraction.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ChvolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_length: usize, // 10
    pub roc_length: usize, // 10
    pub chvol_value: f64,  // 100 · (EMA − EMA[−roc])/EMA[−roc]
    pub chvol_prev: f64,
    pub ema_range: f64, // EMA₁₀(H−L) at last bar
    pub last_close: f64,
    pub chvol_label: String, // EXPANDING / CONTRACTING / NEUTRAL / INSUFFICIENT_DATA
    pub note: String,
}

/// John Bollinger's Bandwidth — `BBW = (upper − lower)/middle` using the
/// standard SMA₂₀ ± 2σ bands. Low readings indicate a "squeeze" of
/// pending volatility expansion; the percentile over a 125-bar window
/// quantifies how extreme the current reading is.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BbwidthSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,  // 20
    pub num_stdev: f64, // 2.0
    pub bbw_value: f64,
    pub bbw_prev: f64,
    pub bbw_percentile: f64, // rank of bbw_value over last 125 bars, 0..100
    pub middle: f64,         // SMA₂₀
    pub upper: f64,
    pub lower: f64,
    pub last_close: f64,
    pub bbw_label: String, // SQUEEZE / LOW / NORMAL / EXPANDED / INSUFFICIENT_DATA
    pub note: String,
}

/// Alexander Elder's Impulse System — colour-codes bars using the sign
/// agreement between a 13-EMA slope and the MACD histogram. GREEN when
/// both rise, RED when both fall, BLUE (neutral/transition) otherwise.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ElderImpulseSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_length: usize, // 13
    pub ema_value: f64,
    pub ema_slope: f64, // EMA − EMA[−1]
    pub macd_hist: f64, // current MACD histogram
    pub macd_hist_prev: f64,
    pub macd_hist_slope: f64, // hist − hist[−1]
    pub last_close: f64,
    pub impulse_label: String, // GREEN / BLUE / RED / INSUFFICIENT_DATA
    pub note: String,
}

/// Roger Altman's Relative Momentum Index — RSI variant applied to the
/// N-bar momentum series `close − close[−N]` instead of the 1-bar change.
/// Tends to lag RSI slightly but produces smoother overbought/oversold
/// signals during strong trends.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RmiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,          // 14
    pub momentum_length: usize, // 5
    pub rmi_value: f64,         // 0..100
    pub rmi_prev: f64,
    pub last_close: f64,
    pub rmi_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// Wilder's Smoothed Moving Average (SMMA / RMA) — a recursive MA with
/// `SMMA_t = (SMMA_{t-1}·(N-1) + price_t) / N`. Equivalent to EMA with
/// `alpha = 1/N` (vs classical EMA's `alpha = 2/(N+1)`), giving it much
/// slower decay and less whipsaw than same-length EMA. Basis of ATR,
/// RSI's average gain/loss, and Williams's Alligator surface.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SmmaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub smma_value: f64,
    pub smma_prev: f64,
    pub deviation_pct: f64, // (close − smma)/smma · 100
    pub last_close: f64,
    pub smma_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Bill Williams's Alligator — three displaced SMMAs of the median price:
/// `jaw = SMMA₁₃(medprice) shifted +8`, `teeth = SMMA₈ shifted +5`,
/// `lips = SMMA₅ shifted +3`. The current-bar values surfaced here are
/// the *shifted-to-today* values, i.e. `jaw[t] = SMMA₁₃(medprice[0..=t-8])`,
/// etc. Label encodes the alligator's state: SLEEPING when the three
/// lines are intertwined; EATING_UP when lips > teeth > jaw and spreading;
/// EATING_DOWN when lips < teeth < jaw and spreading; AWAKENING when
/// crossing. Classic chart-pattern surface in forex/crypto systems.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AlligatorSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub jaw: f64,   // SMMA₁₃, shifted +8
    pub teeth: f64, // SMMA₈,  shifted +5
    pub lips: f64,  // SMMA₅,  shifted +3
    pub jaw_prev: f64,
    pub teeth_prev: f64,
    pub lips_prev: f64,
    pub spread_pct: f64, // (max − min of 3 lines) / last_close · 100
    pub last_close: f64,
    pub alligator_label: String, // EATING_UP / EATING_DOWN / AWAKENING / SLEEPING / INSUFFICIENT_DATA
    pub note: String,
}

/// Larry Connors's Connors RSI — a composite of three momentum
/// components: `CRSI = (RSI₃(close) + RSI₂(streak) + percent_rank(ROC₁, 100))/3`.
/// `streak` is the current up/down streak counter. The percent_rank
/// component measures where today's 1-bar ROC ranks over the last 100
/// bars. Behaves as a mean-reversion signal with sharp extremes — the
/// canonical Connors entry/exit threshold is >90 (short) / <10 (long).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CrsiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rsi_length: usize,    // 3
    pub streak_length: usize, // 2
    pub rank_lookback: usize, // 100
    pub rsi_close: f64,       // RSI₃(close)
    pub rsi_streak: f64,      // RSI₂(streak)
    pub percent_rank: f64,    // 100·rank/lookback
    pub crsi_value: f64,      // mean of the three components
    pub crsi_prev: f64,
    pub last_close: f64,
    pub crsi_label: String, // OVERBOUGHT / BULLISH / NEUTRAL / BEARISH / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// Standard Error Bands — Tim Tillson / Don Fishback channels around a
/// linear regression endpoint fit. Center is the linreg fitted value at
/// `t = N−1`, bands are `center ± k·SE` where
/// `SE = sqrt(Σ(y_i − ŷ_i)² / (N−2))`. Narrower than Bollinger when the
/// fit is good (low residual variance) and wider when price is noisy
/// around the trend. Better mean-reversion signal in trending markets
/// than Bollinger since the center itself captures the trend.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SebSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub num_se: f64,   // 2.0
    pub upper: f64,
    pub middle: f64, // linreg endpoint fit
    pub lower: f64,
    pub bandwidth: f64,    // (upper − lower)/middle
    pub position_pct: f64, // (close − lower)/(upper − lower) · 100
    pub last_close: f64,
    pub seb_label: String, // ABOVE_BAND / UPPER_HALF / NEUTRAL / LOWER_HALF / BELOW_BAND / INSUFFICIENT_DATA
    pub note: String,
}

/// Tushar Chande's Intraday Momentum Index — RSI applied to the
/// **bar-by-bar close-minus-open series** rather than close-minus-prior-
/// close. `IMI = 100·Σ(up_cls-op) / (Σ(up) + Σ|down|)` over N bars.
/// Measures buying vs selling pressure within each bar, making it
/// sensitive to intraday direction. Distinct from RSI (inter-bar), QSTICK
/// (EMA of close-open), and BOP (single-bar scaled close-open).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ImiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub sum_gains: f64,
    pub sum_losses: f64,
    pub imi_value: f64, // 0..100
    pub imi_prev: f64,
    pub last_close: f64,
    pub imi_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// Daryl Guppy's Multiple Moving Average — a fan of twelve EMAs split into
/// a **short-term trader group** (3, 5, 8, 10, 12, 15) and a **long-term
/// investor group** (30, 35, 40, 45, 50, 60). When the short group is
/// above and spread wide and the long group is below and parallel, a
/// strong uptrend is confirmed. Compression in both groups signals an
/// imminent move. `compression_pct` measures the short-group width
/// relative to last close.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GmmaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub short_ema_avg: f64, // mean of 3,5,8,10,12,15
    pub long_ema_avg: f64,  // mean of 30,35,40,45,50,60
    pub short_min: f64,     // min of short group
    pub short_max: f64,     // max of short group
    pub long_min: f64,
    pub long_max: f64,
    pub short_compression_pct: f64, // (short_max − short_min)/last_close · 100
    pub long_compression_pct: f64,
    pub group_gap_pct: f64, // (short_ema_avg − long_ema_avg)/last_close · 100
    pub last_close: f64,
    pub gmma_label: String, // STRONG_UPTREND / UPTREND / COMPRESSION / DOWNTREND / STRONG_DOWNTREND / INSUFFICIENT_DATA
    pub note: String,
}

/// Moving Average Envelope — a simple MA bracketed by **fixed percentage
/// bands** above and below, as distinct from Bollinger (stdev-based) or
/// Keltner (ATR-based). Classical technician's channel: `upper = MA·(1+k)`,
/// `lower = MA·(1−k)`. Position within the envelope is a coarse
/// overbought/oversold gauge.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MaenvSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub pct_band: f64, // 2.5
    pub upper: f64,
    pub middle: f64, // SMA(N)
    pub lower: f64,
    pub bandwidth_pct: f64, // 2 · pct_band (constant, for symmetry)
    pub position_pct: f64,  // (close − lower)/(upper − lower) · 100
    pub last_close: f64,
    pub maenv_label: String, // ABOVE_BAND / UPPER_HALF / NEUTRAL / LOWER_HALF / BELOW_BAND / INSUFFICIENT_DATA
    pub note: String,
}

/// Marc Chaikin's Accumulation/Distribution Line — a cumulative running
/// total of `money flow multiplier × volume`, where
/// `MFM = ((close − low) − (high − close)) / (high − low)`. Tracks whether
/// the bar closes in the upper (accumulation) or lower (distribution) half
/// of its range and weights by volume. Rising ADL with flat/down price is
/// a bullish divergence; falling ADL with flat/up price is bearish.
/// Distinct from OBV (raw signed volume) and CMF (ranged-MFM / ranged-vol
/// ratio): ADL is the cumulative running total of MFM·V.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AdlSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub adl_value: f64, // cumulative total
    pub adl_prev: f64,
    pub adl_sma_length: usize, // 20
    pub adl_sma: f64,          // SMA(ADL, 20)
    pub slope_per_bar: f64,    // OLS slope of last 20 ADL points
    pub last_close: f64,
    pub price_delta_pct: f64, // close over last 20 bars vs adl direction
    pub adl_label: String, // STRONG_ACCUMULATION / ACCUMULATION / NEUTRAL / DISTRIBUTION / STRONG_DISTRIBUTION / INSUFFICIENT_DATA
    pub note: String,
}

/// Adam White's Vertical Horizontal Filter — measures **trendiness vs
/// ranging** of the price series over N bars:
/// `VHF = (HHV_N − LLV_N) / Σ|Δclose|`. High VHF (>0.5) means price is
/// grinding in one direction (trending); low VHF (<0.3) means price is
/// chopping around the same range (ranging). Distinct from ADX (which is
/// a trend strength oscillator on +DI/-DI differences), CHOP (log10 of
/// range/sum-of-TR), and AROON (positional HHV/LLV timing).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VhfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 28
    pub highest_high: f64,
    pub lowest_low: f64,
    pub sum_abs_delta: f64,
    pub vhf_value: f64,
    pub vhf_prev: f64,
    pub last_close: f64,
    pub vhf_label: String, // STRONG_TREND / TREND / NEUTRAL / RANGING / STRONG_RANGING / INSUFFICIENT_DATA
    pub note: String,
}

/// Volume Rate of Change — `VROC = (V_now − V_{now-N}) / V_{now-N} · 100`.
/// Analogous to price ROC but on the volume series. Spikes in VROC mark
/// unusual participation (news, earnings, breakouts); persistent positive
/// VROC with rising price confirms trend. Different from RelVol (which
/// compares current vs long-horizon average) and NVol (current vs 20-day
/// median); VROC is strictly a two-point volume delta.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VrocSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub volume_now: f64,
    pub volume_then: f64,
    pub vroc_value: f64, // pct
    pub vroc_prev: f64,
    pub last_close: f64,
    pub vroc_label: String, // SURGE / ELEVATED / NEUTRAL / QUIET / COLLAPSE / INSUFFICIENT_DATA
    pub note: String,
}

/// KDJ — the Chinese-market Stochastic Oscillator variant built from
/// `RSV_N = 100·(close−LLV_N)/(HHV_N−LLV_N)` over N=9 bars, then
/// `K = EMA_{1/3}(RSV)`, `D = EMA_{1/3}(K)`, and the distinguishing
/// `J = 3·K − 2·D`. J amplifies cross-overs earlier than plain %K/%D,
/// and its extreme prints (J>100 or J<0) are interpreted as aggressive
/// overbought/oversold flags. Distinct from STOCH (, bare %K/%D
/// only), STOCHRSI (, stochastic-of-RSI rather than
/// stochastic-of-price), KST (Pring's Know-Sure-Thing, multi-ROC
/// rate-of-change composite), and WILLR (, inverse %R). KDJ
/// is the one momentum surface that explicitly exposes the amplified
/// J line as a separate field rather than a derived calculation.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KdjSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub stoch_length: usize, // 9 — RSV window
    pub k_smooth: usize,     // 3 — EMA smoothing constant for K and D (α = 1/3 ⇒ period = 5)
    pub rsv: f64,            // raw stochastic %
    pub k_value: f64,        // EMA_{1/3}(RSV)
    pub d_value: f64,        // EMA_{1/3}(K)
    pub j_value: f64,        // 3·K − 2·D
    pub j_prev: f64,
    pub last_close: f64,
    pub kdj_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// QQE — Quantitative Qualitative Estimation, a smoothed RSI-based
/// trend system built by Igor Livshin. Applies 5-bar EMA smoothing to
/// the RSI (default RSI₁₄) to produce `rsi_smoothed`, then computes an
/// adaptive trailing band based on Wilder's MA of ΔRSI: `fast_atr_rsi =
/// Wilder(|ΔRSI|, 14)`, `qqe_fast = rsi_smoothed − 4.236 · fast_atr_rsi_avg`
/// for the lower band, symmetric upper. The trend line is the lagged
/// crossover of these bands. Used as both an early-trend filter and an
/// overbought/oversold gauge — crosses above 50 with trend line below
/// RSI = bullish entry.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct QqeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rsi_length: usize,     // 14
    pub smooth_length: usize,  // 5
    pub qqe_factor: f64,       // 4.236
    pub rsi_value: f64,        // raw RSI
    pub rsi_smoothed: f64,     // EMA(RSI, 5)
    pub fast_atr_rsi_avg: f64, // smoothed Wilder MA of |ΔRSI_smoothed|
    pub upper_band: f64,       // rsi_smoothed + qqe_factor · fast_atr_rsi_avg
    pub lower_band: f64,       // rsi_smoothed − qqe_factor · fast_atr_rsi_avg
    pub qqe_prev: f64,         // prior bar rsi_smoothed
    pub last_close: f64,
    pub qqe_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Martin Pring's Price Momentum Oscillator — a smoothed ROC indicator:
/// `PMO = EMA(EMA(ROC(close,1)·10, 35), 20)` followed by a 10-bar EMA
/// signal line. The heavy triple-smoothing produces a highly reactive
/// but noise-filtered momentum line. Distinct from MACD (EMA₁₂ − EMA₂₆
/// of close), from TRIX (triple-smoothed EMA of close, ), and
/// from PPO (percentage price oscillator); PMO is smoothed-ROC with a
/// signal line, designed for multi-month swing trading.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PmoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub smooth1_length: usize, // 35
    pub smooth2_length: usize, // 20
    pub signal_length: usize,  // 10
    pub pmo_value: f64,
    pub pmo_signal: f64,
    pub pmo_prev: f64,
    pub histogram: f64, // pmo − pmo_signal
    pub last_close: f64,
    pub pmo_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Tushar Chande's Forecast Oscillator — compares the current close to
/// the one-bar-ahead forecast from a linear regression fit over N bars:
/// `CFO = 100 · (close − forecast) / close`. Positive means price is
/// ahead of trend (bullish deviation), negative means behind (bearish
/// deviation). Zero crossings are trend-reversal signals in Chande's
/// systems. Distinct from LINREG (fitted value, ), TSF
/// (projected future value, ), and from PPO / DPO
/// (non-regression momentum). CFO is the one oscillator built as
/// close-minus-regression-forecast as a percentage.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CfoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,  // 14
    pub slope: f64,     // OLS slope
    pub intercept: f64, // OLS intercept
    pub forecast: f64,  // slope·N + intercept (one-bar-ahead)
    pub cfo_value: f64, // 100·(close − forecast)/close
    pub cfo_prev: f64,
    pub last_close: f64,
    pub cfo_label: String, // STRONG_ABOVE_TREND / ABOVE_TREND / NEUTRAL / BELOW_TREND / STRONG_BELOW_TREND / INSUFFICIENT_DATA
    pub note: String,
}

/// Colin Twiggs's Twiggs Money Flow — a smoothed, volume-weighted
/// variant of Chaikin Money Flow. Replaces the bar's full
/// high/low range with a *true range* (max(high, prev_close) −
/// min(low, prev_close)) to correctly handle gap bars, then smooths
/// with an exponential MA rather than a simple sum: TMF tracks
/// cumulative net volume more smoothly than raw CMF and is less
/// jittery on gap-heavy instruments. Twiggs's own default is 21-bar
/// EMA smoothing on both numerator (money flow) and denominator
/// (volume). Distinct from CMF (range-based, simple sum), ADL
/// (cumulative total, not ratio), KLINGER (dual-EMA volume force),
/// and PVT (ROC·volume).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TmfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,       // 21
    pub ema_money_flow: f64, // EMA of money_flow_volume
    pub ema_volume: f64,     // EMA of volume
    pub tmf_value: f64,      // ema_money_flow / ema_volume
    pub tmf_prev: f64,
    pub last_close: f64,
    pub tmf_label: String, // STRONG_INFLOW / INFLOW / NEUTRAL / OUTFLOW / STRONG_OUTFLOW / INSUFFICIENT_DATA
    pub note: String,
}

/// Bill Williams Fractals — 5-bar local-extremum markers. A bullish
/// (up) fractal forms when a bar's high is strictly greater than both
/// the two preceding bars' highs AND the two following bars' highs; a
/// bearish (down) fractal is the symmetric construction on lows. Used
/// as structural S/R pivots and as the building block for Williams's
/// Alligator-based entry/exit rules. Distinct from ZigZag (pct-move
/// threshold) and Pivot Points (floor-trader formula over prior OHLC).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FractalsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub window: usize,             // 5 — 2 left, pivot, 2 right
    pub last_up_high: f64,         // most recent bullish fractal high
    pub last_up_bars_ago: usize,   // bars since last up fractal confirmed
    pub last_down_low: f64,        // most recent bearish fractal low
    pub last_down_bars_ago: usize, // bars since last down fractal confirmed
    pub up_fractal_count: usize,   // total up fractals in scanned window
    pub down_fractal_count: usize, // total down fractals in scanned window
    pub last_close: f64,
    pub fractals_label: String, // UP_RECENT / DOWN_RECENT / BOTH_RECENT / NONE_RECENT / INSUFFICIENT_DATA
    pub note: String,
}

/// Ehlers Inverse Fisher Transform of RSI — rescales RSI to
/// [-5, 5] via `v = 0.1·(RSI − 50)`, smooths with a 9-bar WMA, then
/// applies `ift = (e^{2v} − 1) / (e^{2v} + 1)` to produce a bounded
/// [-1, 1] oscillator. The inverse Fisher transform compresses
/// mid-range values toward zero and expands extremes toward ±1,
/// sharpening reversal signals relative to raw RSI. Crossings of
/// ±0.5 are strong trend-change alerts. Distinct from raw RSI, from
/// STOCHRSI (stochastic of RSI), from QQE (smoothed RSI with
/// adaptive bands, ), and from CRSI (Connors composite,
/// ).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IftRsiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub rsi_length: usize, // 14
    pub wma_length: usize, // 9
    pub rsi_value: f64,    // raw RSI₁₄
    pub v_value: f64,      // WMA₉ of 0.1·(RSI − 50)
    pub ift_value: f64,    // (e^{2v} − 1)/(e^{2v} + 1) ∈ [-1, 1]
    pub ift_prev: f64,
    pub last_close: f64,
    pub ift_rsi_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// MESA Adaptive Moving Average (Ehlers) — a phase-adaptive MA that
/// estimates the dominant cycle period via a simplified Hilbert
/// transform (in-phase and quadrature components) and then sets α
/// adaptively: `α = fast_limit / (period / 2)`, clamped to
/// `[slow_limit, fast_limit]`. The companion FAMA (Following
/// Adaptive MA) is MAMA smoothed with half its α. The fastlimit /
/// slowlimit defaults are 0.5 / 0.05. Distinct from KAMA (Kaufman,
/// efficiency-ratio-based adaptive), from T3 (Tillson triple-DEMA),
/// and from every fixed-α EMA on the shipped MA list.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MamaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_limit: f64, // 0.5
    pub slow_limit: f64, // 0.05
    pub mama_value: f64,
    pub fama_value: f64,
    pub mama_prev: f64,
    pub fama_prev: f64,
    pub alpha: f64,  // current adaptive α
    pub period: f64, // detected dominant cycle period
    pub last_close: f64,
    pub mama_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Ehlers Center of Gravity oscillator — a zero-lag oscillator built
/// as the negative weighted centroid of the last N closes:
/// `COG = -Σ_{i=0..N-1}((i+1)·close_{N-1-i}) / Σ_{i=0..N-1}(close_{N-1-i})`
/// with canonical N=10. Signal line is a 3-bar lagged copy. Ehlers
/// argued that the sign flip and the weighting by recency produce an
/// oscillator that leads traditional momentum by one bar on average.
/// Distinct from every EMA-based oscillator (MACD, TRIX, PMO), from
/// LINREG-based (LINREG/CFO), and from simple ROC.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CogSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 10
    pub cog_value: f64,
    pub cog_signal: f64, // 3-bar lag
    pub cog_prev: f64,
    pub last_close: f64,
    pub cog_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Didi Aguiar's Didi Index — a Brazilian 3-SMA crossover system
/// where three SMAs (short 3, medium 8, long 20) are normalized by
/// dividing by the medium: `short_ratio = short_sma/medium_sma − 1`,
/// `long_ratio = long_sma/medium_sma − 1`. The characteristic "didi
/// needles" pattern fires when short and long cross the zero line
/// from opposite sides — BULL_NEEDLES when short crosses up through
/// zero while long crosses down through zero, and symmetric
/// BEAR_NEEDLES. Between needle events, the ordering of short,
/// medium, and long drives the trend classification. Distinct from
/// every 2-line MA crossover (golden/death cross), from Guppy
/// (GMMA, 12-line fan, ), and from ALLIGATOR (3-line SMMA).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DidiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub short_length: usize,  // 3
    pub medium_length: usize, // 8
    pub long_length: usize,   // 20
    pub short_ratio: f64,     // short/medium − 1
    pub long_ratio: f64,      // long/medium − 1
    pub short_prev: f64,
    pub long_prev: f64,
    pub last_close: f64,
    pub didi_label: String, // BULL_NEEDLES / BULL / NEUTRAL / BEAR / BEAR_NEEDLES / INSUFFICIENT_DATA
    pub note: String,
}

/// Tom DeMark's DeMarker (DeM) — a bounded [0,1] momentum oscillator.
/// Over an N=14 lookback, DeMax[i] = max(high[i]−high[i−1], 0) and
/// DeMin[i] = max(low[i−1]−low[i], 0); summing these and taking
/// `DeM = ΣDeMax / (ΣDeMax + ΣDeMin)` produces an oscillator that
/// weights recent highs vs recent lows, so sustained up-legs push
/// DeM toward 1 and sustained down-legs push it toward 0. Readings
/// above 0.7 flag overbought conditions, below 0.3 oversold.
/// Distinct from RSI (Wilder smoothing of gains/losses on close),
/// from Williams %R (range-position of close), and from STOCHRSI.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DemarkerSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub demax_sum: f64,
    pub demin_sum: f64,
    pub demarker_value: f64, // [0, 1]
    pub demarker_prev: f64,
    pub last_close: f64,
    pub demarker_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// Bill Williams Gator Oscillator — a companion to the Alligator
/// that visualizes how the three shifted SMMAs diverge or
/// converge. `upper = |jaws − teeth|` plotted above zero and
/// `lower = −|teeth − lips|` plotted below zero, where jaws =
/// SMMA₁₃ shifted 8 bars, teeth = SMMA₈ shifted 5, lips = SMMA₅
/// shifted 3. The Gator has four life phases: SLEEPING (both bars
/// small), AWAKENING (bars change color — one growing, one
/// shrinking), EATING (both bars growing — trend feeding), and
/// SATED (both bars shrinking — trend exhausting). Distinct from
/// ALLIGATOR (the raw MA triplet) and from every MA-spread oscillator.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct GatorSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub jaw_length: usize,   // 13
    pub teeth_length: usize, // 8
    pub lips_length: usize,  // 5
    pub upper_bar: f64,      // |jaws − teeth|
    pub lower_bar: f64,      // −|teeth − lips|
    pub upper_prev: f64,
    pub lower_prev: f64,
    pub last_close: f64,
    pub gator_label: String, // SLEEPING / AWAKENING / EATING / SATED / INSUFFICIENT_DATA
    pub note: String,
}

/// Bill Williams Market Facilitation Index (BW MFI) — measures how
/// much price moved per unit of volume: `mfi = (high − low) / volume`
/// (tick-scaled). Williams then classifies each bar by comparing
/// current MFI and volume to the prior bar's values, producing four
/// colored dots: GREEN (MFI up, volume up — genuine strong move),
/// FADE (MFI down, volume down — interest fading), FAKE (MFI up,
/// volume down — false breakout) and SQUAT (MFI down, volume up —
/// indecision battle, often precedes reversal). Distinct from
/// Chaikin's MFI (, based on money-flow volume).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BwMfiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mfi_value: f64, // (high − low) / volume (tick-scaled)
    pub mfi_prev: f64,
    pub volume: f64,
    pub volume_prev: f64,
    pub last_close: f64,
    pub bwmfi_color: String, // GREEN / FADE / FAKE / SQUAT
    pub bwmfi_label: String, // GREEN / FADE / FAKE / SQUAT / INSUFFICIENT_DATA
    pub note: String,
}

/// Volume Weighted Moving Average (VWMA) — a simple moving average
/// of close weighted by volume: `vwma = Σ(close·vol) / Σ(vol)` over
/// N=20. High-volume closes dominate the average, so VWMA diverges
/// from the plain SMA when recent volume spikes above the baseline,
/// providing an institutional-footprint smoother. The VWMA−SMA
/// spread is the core signal: positive when big volume aligns with
/// higher prices, negative when big volume aligns with lower prices.
/// Distinct from VWAP (session-anchored, ) and from every
/// other fixed-length MA (SMA, EMA, HMA, DEMA, ALMA, KAMA, MAMA).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct VwmaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub vwma_value: f64,
    pub sma_value: f64,
    pub vwma_prev: f64,
    pub spread: f64,       // vwma − sma
    pub spread_ratio: f64, // (vwma − sma) / sma
    pub last_close: f64,
    pub vwma_label: String, // BULL / WEAK_BULL / NEUTRAL / WEAK_BEAR / BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Rolling sample standard deviation of close over N=20 with a
/// long-baseline regime classifier. Returns the mean, variance, and
/// sample stddev, plus the 252-day annualized stddev (using daily
/// log-return would change the definition — this snapshot uses
/// price-level stddev). The `regime_label` compares current N=20
/// stddev against a trailing 60-bar stddev: HIGH_VOL when current
/// >1.5× long, LOW_VOL when <0.67×, MID_VOL otherwise. Distinct
/// from EWMAVOL (exponentially-weighted, ), from REALIZED_VOL
/// (return-based), and from Parkinson/Garman-Klass/RS (range-based).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StddevSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,      // 20
    pub long_length: usize, // 60 — baseline
    pub mean: f64,
    pub variance: f64,
    pub stddev: f64,      // sample stddev of close over N
    pub stddev_long: f64, // sample stddev of close over long_length
    pub cv: f64,          // coefficient of variation = stddev / mean
    pub annualized: f64,  // stddev · sqrt(252)
    pub last_close: f64,
    pub regime_label: String, // HIGH_VOL / MID_VOL / LOW_VOL / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 60 — WMA / RAINBOW / MESA_SINE / FRAMA / IBS ─────────────

/// Weighted Moving Average (WMA) — a linearly-weighted moving
/// average where weights increase from 1 (oldest) to N (newest):
/// `wma = Σ(price[i] · (i+1)) / Σ(i+1)` for i in 0..N-1 with N=20.
/// WMA puts more emphasis on recent bars than SMA but less than
/// EMA, producing a smoother line that still reacts to recent
/// price changes. Distinct from SMA (equal weights), EMA
/// (exponential decay), HMA (WMA of 2·WMA(n/2) − WMA(n)),
/// DEMA/TEMA (EMA recursion), and ALMA (Gaussian kernel).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct WmaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub wma_value: f64,
    pub wma_prev: f64,
    pub sma_value: f64,
    pub spread: f64,     // close − wma
    pub spread_pct: f64, // (close − wma) / wma
    pub last_close: f64,
    pub wma_label: String, // BULL / WEAK_BULL / NEUTRAL / WEAK_BEAR / BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Rainbow MA Oscillator — a multi-level recursive SMA stack
/// where `r_1 = SMA(close, 2)`, `r_2 = SMA(r_1, 2)`, ..., `r_10 =
/// SMA(r_9, 2)`. The 10 levels create a "rainbow" fan around
/// price, and the oscillator reports the highest-high minus
/// lowest-low across the levels (the rainbow width) along with
/// the fan's current center. A wide rainbow means strong trend
/// (levels spread apart); a narrow rainbow means consolidation
/// (levels bunched tightly). Distinct from GMMA (Guppy's
/// 12-line EMA fan, ) which uses EMAs of varying lengths
/// rather than recursive 2-bar SMAs.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RainbowSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub levels: usize,          // 10
    pub highest_level: f64,     // max across r_1..r_10
    pub lowest_level: f64,      // min across r_1..r_10
    pub rainbow_width: f64,     // highest - lowest
    pub rainbow_width_pct: f64, // width / center
    pub center_value: f64,      // mean of all levels
    pub r1: f64,
    pub r5: f64,
    pub r10: f64,
    pub last_close: f64,
    pub rainbow_label: String, // STRONG_TREND / TRENDING / CONSOLIDATING / INSUFFICIENT_DATA
    pub note: String,
}

/// Ehlers MESA Sine Wave — uses a simplified Hilbert-transform
/// phase estimator to detect the dominant cycle phase and emits
/// `sine = sin(phase)` and `lead_sine = sin(phase + 45°)`. When
/// the sine crosses above the lead_sine, a cycle-bottom buy is
/// flagged; when it crosses below, a cycle-top sell is flagged.
/// In trending markets the two lines separate and fail to cross,
/// producing no signals — a useful regime filter in itself.
/// Distinct from MAMA (phase-adaptive MA, ), FISHER
/// (probability Gaussianization, ), and COG (weighted
/// centroid, ); MESA_SINE focuses on cycle phase rather
/// than value or momentum.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MesaSineSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: f64,     // detected dominant cycle period in bars
    pub phase_rad: f64,  // current phase angle
    pub sine_value: f64, // sin(phase)
    pub lead_sine: f64,  // sin(phase + π/4)
    pub sine_prev: f64,
    pub lead_prev: f64,
    pub last_close: f64,
    pub mesa_label: String, // CYCLE_BUY / CYCLE_SELL / TRENDING / NEUTRAL / INSUFFICIENT_DATA
    pub note: String,
}

/// Fractal Adaptive Moving Average (FRAMA) — Ehlers's adaptive MA
/// where the smoothing α is driven by the fractal dimension D of
/// the price series over the last N bars. Computed by dividing
/// N=16 into two halves, measuring the H-L range of each half
/// and the combined range, then solving for D from the Hurst-like
/// ratio; α = exp(-4.6·(D − 1)). Strong trends (D near 1.0)
/// yield α ≈ 1 (fast-following); choppy markets (D near 2.0)
/// yield α near 0.01 (heavy smoothing). Distinct from KAMA
/// (efficiency-ratio adaptive, ), VIDYA (volatility-index
/// adaptive, ), MAMA (Hilbert-phase adaptive, ),
/// and T3 (Tillson triple-DEMA, ).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct FramaSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,    // 16
    pub fractal_dim: f64, // D ∈ [1, 2]
    pub alpha: f64,       // exp(-4.6·(D-1))
    pub frama_value: f64,
    pub frama_prev: f64,
    pub spread: f64, // close - frama
    pub last_close: f64,
    pub frama_label: String, // STRONG_TREND / TREND / CHOP / INSUFFICIENT_DATA
    pub note: String,
}

/// Internal Bar Strength (IBS) — the position of close within the
/// current bar's high/low range: `ibs = (close − low) / (high −
/// low)`, bounded on `[0, 1]`. A 14-bar SMA smooths the raw
/// reading. Values near 1 indicate close at the high (bullish
/// conviction within the bar); values near 0 indicate close at
/// the low (bearish conviction). IBS is a mean-reversion favorite
/// — high IBS readings (>0.8) often precede short-term
/// pullbacks, low IBS (<0.2) often precede bounces. Distinct from
/// STOCH (%K over N-bar HHV/LLV, ) which spans multiple
/// bars, and from every momentum oscillator; IBS is a single-bar
/// position metric.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct IbsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,     // 14
    pub ibs_raw: f64,      // current bar's IBS
    pub ibs_smoothed: f64, // 14-bar SMA of IBS
    pub ibs_prev: f64,
    pub last_high: f64,
    pub last_low: f64,
    pub last_close: f64,
    pub ibs_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 61 — LAGUERRE_RSI / ZIGZAG / PGO / HT_TRENDLINE / MIDPOINT ──

/// Ehlers Laguerre RSI — a bounded [0, 1] oscillator built from
/// Ehlers's 4-stage Laguerre filter (γ=0.5). The 4-stage filter
/// smooths the close and produces L0, L1, L2, L3 intermediate
/// outputs; the Laguerre RSI is then computed from the count of
/// upward differences vs total differences across the stages,
/// yielding a cleaner oscillator than classic RSI with no
/// divergence false signals near extremes. Distinct from RSI
/// (Wilder smoothing of gains/losses, ), STOCHRSI,
/// CRSI (Connors, ), QQE, and IFT_RSI.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LaguerreRsiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub gamma: f64, // 0.5
    pub l0: f64,
    pub l1: f64,
    pub l2: f64,
    pub l3: f64,
    pub laguerre_rsi: f64, // [0, 1]
    pub laguerre_rsi_prev: f64,
    pub last_close: f64,
    pub lrsi_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// ZigZag pattern detector — reports the most recent confirmed
/// swing high and swing low using a fixed percentage threshold
/// (default 5%). A new pivot forms when price reverses by at least
/// threshold_pct from the prior extreme. The snapshot emits the
/// last high pivot (value + bars_ago), the last low pivot, the
/// active leg direction (UP/DOWN), and the projected reversal
/// level. Distinct from FRACTALS (, Bill Williams 5-bar
/// strict peaks) and from PIVOTS (, prior-session math),
/// which use fundamentally different construction — ZigZag is a
/// %-threshold reversal detector.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ZigzagSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub threshold_pct: f64, // 5.0 (percent)
    pub last_high_value: f64,
    pub last_high_bars_ago: usize,
    pub last_low_value: f64,
    pub last_low_bars_ago: usize,
    pub current_leg: String, // UP / DOWN / INSUFFICIENT_DATA
    pub reversal_level: f64, // threshold from active extreme
    pub last_close: f64,
    pub zigzag_label: String, // UP_LEG / DOWN_LEG / AT_REVERSAL / INSUFFICIENT_DATA
    pub note: String,
}

/// Mark Johnson's Pretty Good Oscillator (PGO) — measures the
/// distance of the current close from an N-period SMA expressed
/// in multiples of the N-period ATR:
/// `pgo = (close − SMA(close, N)) / EMA(TR, N)` with N=14. Extreme
/// readings of ±3 were found to be rare and persistent, making
/// PGO a trend-following signal rather than mean-reversion — the
/// "pretty good" name reflects Johnson's empirical observation that
/// it filters noise better than raw ROC. Distinct from ROC
/// (unscaled price change), PPO (percentage-scaled MACD, ),
/// and RSI/STOCH (bounded oscillators) because PGO's scaling is by
/// volatility, not percent.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PgoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub sma_value: f64,
    pub atr_value: f64, // EMA of TR over N
    pub pgo_value: f64, // (close - sma) / atr
    pub pgo_prev: f64,
    pub last_close: f64,
    pub pgo_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Hilbert Transform Instantaneous Trendline (HT_TRENDLINE) — a
/// smoothed trendline based on the dominant cycle period derived
/// from Ehlers's Hilbert-transform homodyne discriminator. Unlike
/// MAMA which outputs an adaptive MA proper,
/// HT_TRENDLINE reports the `trendline = WMA(close, period)` over
/// the detected cycle period — a lag-matched smoother that
/// follows the dominant trend without the adaptive α rescaling.
/// Distinct from MAMA (adaptive α), FRAMA (fractal-
/// dimension α), and every fixed-length smoother.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HtTrendlineSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: f64, // detected dominant cycle
    pub trendline_value: f64,
    pub trendline_prev: f64,
    pub spread: f64, // close - trendline
    pub spread_pct: f64,
    pub last_close: f64,
    pub ht_label: String, // BULL / WEAK_BULL / NEUTRAL / WEAK_BEAR / BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// Midpoint of N — `midpoint = (HHV(N) + LLV(N)) / 2` emitting
/// the midpoint of the N-bar range along with the HHV, LLV, and
/// the close's position within the range. N=14. TA-Lib's MIDPOINT
/// function; useful as a simple anchor for detecting where price
/// sits relative to the most recent trading range. Distinct from
/// Donchian channel (, raw HHV/LLV bands), from SMA, and
/// from pivot systems because it uses only HHV+LLV
/// extremes rather than OHLC4 or session math.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MidpointSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub hhv: f64,
    pub llv: f64,
    pub midpoint: f64, // (HHV + LLV) / 2
    pub midpoint_prev: f64,
    pub close_position: f64, // (close - LLV) / (HHV - LLV), bounded [0, 1]
    pub last_close: f64,
    pub midpoint_label: String, // UPPER / NEAR_UPPER / MIDRANGE / NEAR_LOWER / LOWER / INSUFFICIENT_DATA
    pub note: String,
}

/// Donald Dorsey's Mass Index — an exhaustion/reversal indicator
/// built from a 25-period sum of the ratio of 9-period EMA(H−L) to
/// 9-period EMA of that EMA (EMA of EMA). A classic "reversal
/// bulge" crosses above 27 then below 26.5 signalling a likely
/// reversal independent of direction. Distinct from ATR (range
/// magnitude, ) because Mass Index is a unitless ratio of
/// range smoothings and from CHOP because it measures
/// range expansion/contraction via nested EMAs rather than
/// high-low efficiency.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MassIndexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ema_len: usize,     // 9
    pub sum_len: usize,     // 25
    pub ema_range: f64,     // EMA(H-L) latest
    pub ema_ema_range: f64, // EMA of EMA(H-L) latest
    pub ratio: f64,         // ema_range / ema_ema_range
    pub mass_index: f64,    // Σ(ratio) over sum_len
    pub mass_index_prev: f64,
    pub last_close: f64,
    pub mass_label: String, // REVERSAL_BULGE / ELEVATED / NEUTRAL / COMPRESSED / INSUFFICIENT_DATA
    pub note: String,
}

/// Normalized ATR (NATR) — TA-Lib's `natr = 100 × ATR(N) / close`,
/// expressing Wilder's Average True Range as a percentage of the
/// closing price. This makes ATR scale-invariant so it can be
/// compared across symbols of different price levels (a $5 ATR
/// means different things for a $10 stock vs a $500 stock).
/// Distinct from plain ATR (raw dollar volatility, ) and
/// from stddev-based vols (STDDEV) because NATR uses the
/// true range directly rather than log return variance.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct NatrSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,   // 14
    pub atr_value: f64,  // Wilder ATR(14)
    pub natr_value: f64, // 100 × ATR / close
    pub natr_prev: f64,
    pub last_close: f64,
    pub natr_label: String, // HIGH_VOL / ELEVATED / NORMAL / LOW_VOL / INSUFFICIENT_DATA
    pub note: String,
}

/// John Carter's TTM Squeeze — a regime flag indicating whether
/// Bollinger Bands (2σ) fit entirely inside Keltner Channels
/// (1.5×ATR). When BB ⊂ KC, volatility is compressed and a
/// breakout is imminent ("squeeze on"). When BB expands outside
/// KC, the squeeze fires ("squeeze off") and directional
/// momentum typically follows. Paired with a linear-regression
/// histogram to indicate breakout direction (up vs down).
/// Distinct from BBW (Bollinger Band Width regime) and
/// from Keltner (standalone bands) because TTM Squeeze
/// tests the geometric relation between both systems.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TtmSqueezeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub bb_upper: f64,
    pub bb_lower: f64,
    pub kc_upper: f64,    // middle + 1.5×ATR
    pub kc_lower: f64,    // middle − 1.5×ATR
    pub squeeze_on: bool, // BB ⊂ KC
    pub momentum: f64,    // linreg of (close - midrange)
    pub momentum_prev: f64,
    pub last_close: f64,
    pub squeeze_label: String, // SQUEEZE_ON / FIRE_UP / FIRE_DOWN / NEUTRAL / INSUFFICIENT_DATA
    pub note: String,
}

/// Alexander Elder's Force Index — `force = volume × (close −
/// close_prev)`, smoothed by a 13-period EMA. Measures the buying
/// / selling pressure behind price moves: strong positive Force
/// means high volume on an up move (bullish conviction); strong
/// negative means heavy selling. Zero-line crossings flag
/// momentum shifts; extreme readings warn of exhaustion.
/// Distinct from OBV (cumulative sign-weighted volume, )
/// because Force Index weights by the size of the price change
/// not just the direction, and from CMF (money-flow-multiplier,
/// ) which uses H/L rather than bar-over-bar close change.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ForceIndexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,  // 13
    pub force_raw: f64, // latest raw force
    pub force_ema: f64, // EMA-smoothed
    pub force_ema_prev: f64,
    pub last_close: f64,
    pub last_volume: f64,
    pub force_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// True Range (raw, single-bar) — TA-Lib's TRANGE function:
/// `tr = max(H − L, |H − C_prev|, |L − C_prev|)`. The single-bar
/// volatility measure that underlies Wilder's ATR but
/// reports the current bar's TR directly without any smoothing.
/// Useful for gap-aware bar-size comparisons and for building
/// custom volatility systems. Distinct from ATR (N-period EMA of
/// TR) and from the bar's raw range (H − L) which ignores gaps.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct TrangeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub trange_value: f64, // latest true range
    pub trange_prev: f64,
    pub mean_trange_20: f64, // 20-bar mean TR
    pub trange_ratio: f64,   // latest / mean_20 (expansion ratio)
    pub last_high: f64,
    pub last_low: f64,
    pub last_close: f64,
    pub prev_close: f64,
    pub trange_label: String, // EXPANSION / NORMAL / CONTRACTION / INSUFFICIENT_DATA
    pub note: String,
}

/// Linear Regression Slope (LINEARREG_SLOPE) — TA-Lib's linreg slope
/// function: the least-squares slope of an N-bar linear regression
/// on close prices, in price-units-per-bar. Distinct from TSF (value
/// of linear regression line at the current bar, ) and from
/// LINEARREG (the regression line value) because this emits just
/// the slope coefficient β. A positive slope indicates trending up;
/// the magnitude captures the rate of trend acceleration.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LinearregSlopeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub slope: f64,    // price/bar
    pub slope_prev: f64,
    pub slope_pct: f64, // slope / last_close × 100 (bar-pct)
    pub last_close: f64,
    pub slope_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// Hilbert Dominant Cycle Period (HT_DCPERIOD) — Ehlers's Hilbert-
/// transform homodyne discriminator applied to close prices to
/// detect the dominant cycle period in bars. Distinct from HT_TRENDLINE
/// which uses the period to drive a WMA smoother, and from
/// MESA_SINE / MAMA which use the period for adaptive α rescaling.
/// This snapshot emits just the detected period itself — useful for
/// choosing adaptive parameters on other indicators, or for regime
/// detection where cycle-length dynamics matter.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HtDcperiodSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: f64, // dominant cycle in bars (smoothed)
    pub period_prev: f64,
    pub period_min_64: f64, // min over last 64 bars
    pub period_max_64: f64, // max over last 64 bars
    pub last_close: f64,
    pub period_label: String, // VERY_SHORT / SHORT / MEDIUM / LONG / VERY_LONG / INSUFFICIENT_DATA
    pub note: String,
}

/// Hilbert Trend-vs-Cycle Mode (HT_TRENDMODE) — Ehlers's regime
/// classifier derived from the dominant cycle period and discriminator
/// stability: 0 = cycle mode (mean-reverting), 1 = trend mode
/// (directional). Paired with a "lock-in" duration counter showing how
/// many bars the current regime has persisted. Distinct from HT_DCPERIOD
/// which emits the period itself — this emits the binary regime flag.
/// Useful for enabling/disabling mean-reversion vs trend-following
/// strategies in real time.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HtTrendmodeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub trendmode: i32, // 0 or 1
    pub trendmode_prev: i32,
    pub lock_in_bars: usize, // how many bars in current mode
    pub period: f64,         // concurrent detected period
    pub last_close: f64,
    pub mode_label: String, // TREND / CYCLE / INSUFFICIENT_DATA
    pub note: String,
}

/// Acceleration Bands (ACCBANDS) — Price Headley's ACCBANDS:
/// `acc_upper = H × (1 + 4×(H−L)/(H+L))`, `acc_lower = L × (1 − 4×(H−L)/(H+L))`,
/// each SMA-smoothed over N=20 periods. The bands are a price-envelope
/// that expands with volatility relative to price level. Breakouts
/// outside the bands signal trend strength. Distinct from Bollinger
/// (σ-based), Keltner (ATR-based), and Donchian (HHV/LLV) because
/// ACCBANDS scales by the range-to-midprice ratio.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AccbandsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 20
    pub acc_upper: f64,
    pub acc_middle: f64, // SMA of close
    pub acc_lower: f64,
    pub width: f64,    // (upper - lower) / middle
    pub position: f64, // (close - lower) / (upper - lower), [0,1]
    pub last_close: f64,
    pub accbands_label: String, // BREAKOUT_UP / UPPER / MID / LOWER / BREAKOUT_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// Fast Stochastic (STOCHF) — TA-Lib's STOCHF: the unsmoothed
/// stochastic oscillator pair (%K, %D) without the inner 3-bar
/// smoothing of slow STOCH. `%K = 100 × (C − LLV(N)) / (HHV(N) − LLV(N))`
/// with N=14, and %D = SMA(%K, 3). Distinct from STOCH (slow
/// stochastic with inner smoothing applied), STOCHRSI (applied
/// to RSI), and SMI (scaled MIDPRICE). Faster, noisier, more
/// responsive to immediate price action.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct StochfSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,   // 14
    pub d_period: usize, // 3
    pub fastk: f64,      // 0..100
    pub fastk_prev: f64,
    pub fastd: f64, // SMA(fastk, 3)
    pub fastd_prev: f64,
    pub last_close: f64,
    pub stochf_label: String, // OVERBOUGHT / BULL / NEUTRAL / BEAR / OVERSOLD / INSUFFICIENT_DATA
    pub note: String,
}

/// Linear Regression (LINEARREG) — TA-Lib's LINEARREG: the fitted value
/// `y_hat(t) = slope·(N-1) + intercept` of the least-squares regression
/// of close over the last N=14 bars, reporting the endpoint of the
/// fitted line. Distinct from LINEARREG_SLOPE (raw slope),
/// LINEARREG_ANGLE (slope→angle), and LINEARREG_INTERCEPT (y at t=0)
/// because LINEARREG reports the projected endpoint of the line — the
/// most recent fitted value, which is the closest fitted approximation
/// of the current close and a common baseline for mean-reversion
/// signals (close − fitted).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LinearregSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub fitted: f64,   // y_hat at current bar
    pub fitted_prev: f64,
    pub residual: f64,     // close - fitted
    pub residual_pct: f64, // residual / close × 100
    pub last_close: f64,
    pub linearreg_label: String, // ABOVE_TREND / NEAR_TREND / BELOW_TREND / INSUFFICIENT_DATA
    pub note: String,
}

/// Linear Regression Angle (LINEARREG_ANGLE) — TA-Lib's
/// LINEARREG_ANGLE: `atan(slope) · 180/π`, converting the raw slope
/// coefficient to an angle in degrees (-90° to +90°). Useful for
/// comparing slope magnitudes across different price scales in a
/// bounded unit. Distinct from LINEARREG_SLOPE (raw units-per-bar)
/// because angle normalizes relative to the price scale.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LinearregAngleSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize,  // 14
    pub slope: f64,     // raw (price-units per bar)
    pub angle_deg: f64, // atan(slope) · 180/π
    pub angle_deg_prev: f64,
    pub last_close: f64,
    pub angle_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// Hilbert Dominant Cycle Phase (HT_DCPHASE) — TA-Lib's HT_DCPHASE
/// reuses the Ehlers homodyne discriminator pipeline (same as
/// HT_DCPERIOD, ) and reports the phase of the dominant cycle
/// at the current bar in degrees (0..360°). Useful for timing cycle
/// turns — phase 0° is the cycle bottom, 180° is the top. Distinct
/// from HT_DCPERIOD (cycle length in bars) and HT_TRENDMODE (trend
/// vs cycle regime) because it reports the cycle's current position
/// within its rotation rather than its length or regime.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HtDcphaseSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub phase_deg: f64, // 0..360°
    pub phase_deg_prev: f64,
    pub phase_delta: f64, // smoothed derivative
    pub period: f64,      // dominant cycle period in bars
    pub last_close: f64,
    pub phase_label: String, // CYCLE_BOTTOM / RISING / CYCLE_TOP / FALLING / INSUFFICIENT_DATA
    pub note: String,
}

/// Hilbert Sine Wave (HT_SINE) — TA-Lib's HT_SINE emits two lines:
/// `sine = sin(phase)` and `leadsine = sin(phase + 45°)`. Crossovers
/// of sine/leadsine identify cycle turns in advance — leadsine
/// crossing above sine signals an imminent cycle bottom, and crossing
/// below signals an imminent top. Distinct from HT_DCPHASE (raw
/// phase) because HT_SINE plots the sine-transformed phase, letting
/// you visualize and cross-trigger cycle turns as bounded signals in
/// [-1, +1].
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HtSineSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub sine: f64, // -1..1
    pub sine_prev: f64,
    pub leadsine: f64, // sin(phase + 45°)
    pub leadsine_prev: f64,
    pub crossover: i32, // +1 leadsine above (bottom turn), -1 below (top turn), 0 none
    pub period: f64,
    pub last_close: f64,
    pub sine_label: String, // CYCLE_TURN_UP / BULL / NEUTRAL / BEAR / CYCLE_TURN_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// Hilbert Phasor Components (HT_PHASOR) — TA-Lib's HT_PHASOR emits
/// the raw in-phase (I) and quadrature (Q) components of the
/// analytic signal produced by the Hilbert transform of smoothed
/// price. Magnitude `sqrt(I² + Q²)` is the instantaneous cycle
/// amplitude; `atan2(Q, I)` is the phase. Distinct from HT_DCPHASE
/// (transforms I/Q into the phase angle) and HT_SINE (sine of the
/// phase) because HT_PHASOR reports the raw I/Q components useful
/// for custom cycle analysis and filter design.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HtPhasorSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub i_comp: f64, // in-phase component
    pub q_comp: f64, // quadrature component
    pub i_prev: f64,
    pub q_prev: f64,
    pub magnitude: f64, // sqrt(I² + Q²)
    pub phase_deg: f64, // atan2(Q, I) · 180/π
    pub last_close: f64,
    pub phasor_label: String, // STRONG_CYCLE / CYCLE / WEAK_CYCLE / INSUFFICIENT_DATA
    pub note: String,
}

/// MIDPRICE — TA-Lib MIDPRICE function: midpoint between the
/// highest high and the lowest low over an N-bar window (default 14).
/// Distinct from MIDPOINT (close-based midpoint, ) and from
/// Donchian (which exposes both bands separately) because MIDPRICE
/// reports the HH/LL midpoint as a single line anchored to the bar
/// range rather than close.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MidpriceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub length: usize, // 14
    pub midprice: f64, // (HHV(H, 14) + LLV(L, 14)) / 2
    pub midprice_prev: f64,
    pub hhv: f64,
    pub llv: f64,
    pub last_close: f64,
    pub position: f64,          // (close - llv) / (hhv - llv), 0..1
    pub midprice_label: String, // NEAR_HIGH / ABOVE_MID / AT_MID / BELOW_MID / NEAR_LOW / INSUFFICIENT_DATA
    pub note: String,
}

/// APO — TA-Lib Absolute Price Oscillator: `EMA_fast(close)
/// − EMA_slow(close)` with defaults fast=12, slow=26. Distinct from
/// PPO (percentage APO: `(fast − slow) / slow × 100`) and from MACD
/// (APO + signal line + histogram) because APO reports the raw
/// difference in price units, preserving absolute magnitude.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ApoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize, // 12
    pub slow_period: usize, // 26
    pub apo: f64,           // fast_ema - slow_ema
    pub apo_prev: f64,
    pub fast_ema: f64,
    pub slow_ema: f64,
    pub last_close: f64,
    pub apo_label: String, // STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// MOM — TA-Lib raw momentum: `close − close[n−period]`
/// over a 10-bar default lookback. Distinct from ROC (percentage: mom
/// / close[n−period] × 100) and from MOMENTUM_12_1 (composite 12m−1m
/// factor score) because MOM reports the raw price delta in currency
/// units — useful as a pre-scaled input for custom oscillator
/// smoothing or absolute-distance filters.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MomSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 10
    pub mom: f64,      // close - close[n - period]
    pub mom_prev: f64,
    pub mom_pct: f64, // mom / close × 100
    pub last_close: f64,
    pub mom_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// SAREXT — TA-Lib Extended Parabolic SAR with configurable
/// asymmetric long/short acceleration factors and an optional forced
/// start trend. Distinct from PSAR (fixed 0.02/0.02/0.20) in
/// that SAREXT exposes separate af_init/af_step/af_max for long vs
/// short regimes, enabling traders to tune the trailing stop's
/// aggressiveness differently on each side of the trade (typical for
/// instruments with asymmetric volatility).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SarextSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub start_value: f64, // 0 = auto; positive forces start long; negative forces start short
    pub af_init_long: f64, // 0.02
    pub af_step_long: f64, // 0.02
    pub af_max_long: f64, // 0.20
    pub af_init_short: f64, // 0.02
    pub af_step_short: f64, // 0.02
    pub af_max_short: f64, // 0.20
    pub sar_value: f64,
    pub extreme_point: f64,
    pub acceleration_factor: f64,
    pub trend_is_up: bool,
    pub bars_in_trend: usize,
    pub distance_pct: f64,
    pub last_close: f64,
    pub sarext_label: String, // STRONG_UP / UP / STRONG_DOWN / DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// ADXR — TA-Lib Average Directional Movement Rating:
/// `(ADX_now + ADX[n − period]) / 2` over a 14-bar default lookback.
/// Distinct from ADX (point-in-time directional movement strength)
/// because ADXR smooths ADX with its lagged value to emphasise trend
/// persistence — a rising ADXR while ADX is flat signals a maturing
/// trend, while falling ADXR confirms trend exhaustion.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AdxrSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub adx_now: f64,
    pub adx_prior: f64, // adx[n - period]
    pub adxr: f64,      // (adx_now + adx_prior) / 2
    pub adxr_prev: f64,
    pub last_close: f64,
    pub adxr_label: String, // STRONG_TREND / TREND / WEAK_TREND / NO_TREND / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 66 ──────────────────────────────────────────────────────
/// TA-Lib AVGPRICE — `(open + high + low + close) / 4`.
/// Simplest price-transform primitive: the four-component OHLC average.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AvgpriceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub avgprice: f64, // (O+H+L+C) / 4
    pub avgprice_prev: f64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub delta_pct: f64,         // (avgprice - close) / close * 100
    pub avgprice_label: String, // ABOVE_CLOSE / NEAR_CLOSE / BELOW_CLOSE / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MEDPRICE — `(high + low) / 2`.
/// Range-midpoint primitive — the simple median of the bar's range.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MedpriceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub medprice: f64, // (H+L) / 2
    pub medprice_prev: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub delta_pct: f64,         // (medprice - close) / close * 100
    pub medprice_label: String, // ABOVE_MID / AT_MID / BELOW_MID / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib TYPPRICE — `(high + low + close) / 3`.
/// Typical price — used as the input for CCI and several VWAP variants.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TypPriceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub typprice: f64, // (H+L+C) / 3
    pub typprice_prev: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub delta_pct: f64,         // (typprice - close) / close * 100
    pub typprice_label: String, // ABOVE_CLOSE / NEAR_CLOSE / BELOW_CLOSE / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib WCLPRICE — `(high + low + 2 × close) / 4`.
/// Weighted close — double-weights the close price for close-biased transforms.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WclPriceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub wclprice: f64, // (H+L+2C) / 4
    pub wclprice_prev: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub delta_pct: f64,         // (wclprice - close) / close * 100
    pub wclprice_label: String, // ABOVE_CLOSE / NEAR_CLOSE / BELOW_CLOSE / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib VARIANCE — sample variance of close over N bars.
/// Statistical variance primitive: `σ² = Σ(x − μ)² / N` (population form, matching TA-Lib default).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VarianceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 5 (TA-Lib default)
    pub mean: f64,
    pub variance: f64, // population variance Σ(x-μ)²/N
    pub variance_prev: f64,
    pub stddev: f64, // sqrt(variance)
    pub cv: f64,     // stddev / |mean| × 100 (coefficient of variation %)
    pub last_close: f64,
    pub variance_label: String, // HIGH_VOL / ELEVATED / NORMAL / LOW_VOL / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 67 ──────────────────────────────────────────────────────
/// TA-Lib PLUS_DI — Wilder's Positive Directional Indicator.
/// `+DI = 100 × (Wilder-smoothed +DM) / ATR` over 14-bar default.
/// Measures upward directional movement strength: paired with −DI it
/// forms the crossover signal under Wilder's Directional Movement System
/// and feeds DX / ADX / ADXR (Rounds 31, 65).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PlusDiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub plus_di: f64,
    pub plus_di_prev: f64,
    pub minus_di: f64, // for crossover context
    pub atr: f64,      // Wilder-smoothed true range
    pub last_close: f64,
    pub plus_di_label: String, // BULL_DOMINANT / BULL_LEAN / NEUTRAL / BEAR_LEAN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MINUS_DI — Wilder's Negative Directional Indicator.
/// `−DI = 100 × (Wilder-smoothed −DM) / ATR` over 14-bar default.
/// Measures downward directional movement strength; mirror primitive of
/// +DI under Wilder's DM System. Distinct from +DI's bull framing.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MinusDiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub minus_di: f64,
    pub minus_di_prev: f64,
    pub plus_di: f64, // for crossover context
    pub atr: f64,     // Wilder-smoothed true range
    pub last_close: f64,
    pub minus_di_label: String, // BEAR_DOMINANT / BEAR_LEAN / NEUTRAL / BULL_LEAN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib PLUS_DM — Wilder's raw Positive Directional Movement.
/// `+DM_t = max(0, H_t − H_{t−1})` only when that up-move exceeds
/// `L_{t−1} − L_t`. Wilder-smoothed via the standard recursion
/// `S_t = S_{t−1} − S_{t−1}/period + +DM_t`, the direct upstream of
/// +DI (divides by ATR to normalise to 0–100).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct PlusDmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,         // 14
    pub plus_dm_raw: f64,      // latest bar's raw +DM
    pub plus_dm_smoothed: f64, // Wilder-smoothed Σ(+DM)
    pub plus_dm_smoothed_prev: f64,
    pub up_move: f64,   // H_t − H_{t−1}
    pub down_move: f64, // L_{t−1} − L_t
    pub last_close: f64,
    pub plus_dm_label: String, // BULL_PRESSURE / BULL_SOFT / NEUTRAL / BEAR_PRESSURE / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MINUS_DM — Wilder's raw Negative Directional Movement.
/// `−DM_t = max(0, L_{t−1} − L_t)` only when that down-move exceeds
/// `H_t − H_{t−1}`. Wilder-smoothed; direct upstream of −DI.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MinusDmSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize,          // 14
    pub minus_dm_raw: f64,      // latest bar's raw −DM
    pub minus_dm_smoothed: f64, // Wilder-smoothed Σ(−DM)
    pub minus_dm_smoothed_prev: f64,
    pub up_move: f64,   // H_t − H_{t−1}
    pub down_move: f64, // L_{t−1} − L_t
    pub last_close: f64,
    pub minus_dm_label: String, // BEAR_PRESSURE / BEAR_SOFT / NEUTRAL / BULL_PRESSURE / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib DX — Wilder's Directional Movement Index.
/// `DX = 100 × |+DI − −DI| / (+DI + −DI)` — the unsmoothed directional
/// dispersion that feeds ADX (Round 31) and ADXR (Round 65) via a
/// further Wilder smoothing. DX alone is a raw directional-purity
/// indicator: high when +DI and −DI diverge, regardless of sign.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DxSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub dx: f64,       // 100·|+DI − −DI|/(+DI + −DI)
    pub dx_prev: f64,
    pub plus_di: f64,
    pub minus_di: f64,
    pub last_close: f64,
    pub dx_label: String, // STRONG_DIR / DIR / WEAK_DIR / NO_DIR / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 68 ──────────────────────────────────────────────────────
/// TA-Lib ROC — raw Rate of Change `close_t − close_{t−n}` (period 10).
/// Raw price delta; distinct from ROCP (percentage) and ROCR (ratio).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RocSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 10
    pub roc: f64,      // close_t − close_{t-n}
    pub roc_prev: f64,
    pub close_now: f64,
    pub close_lag: f64, // close_{t-n}
    pub last_close: f64,
    pub roc_label: String, // STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib ROCP — Rate of Change Percentage `(close_t − close_{t−n}) / close_{t−n}`.
/// The "percentage change" form used widely in risk-return math.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RocpSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 10
    pub rocp: f64,     // (close_t − close_{t-n}) / close_{t-n}  (unitless)
    pub rocp_prev: f64,
    pub rocp_pct: f64, // rocp × 100 (percent display)
    pub close_now: f64,
    pub close_lag: f64,
    pub last_close: f64,
    pub rocp_label: String, // STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib ROCR — Rate of Change Ratio `close_t / close_{t−n}` (period 10).
/// Ratio-form rate of change — 1.0 is no change, >1 up, <1 down.
/// Direct input for compounding return aggregations.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct RocrSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 10
    pub rocr: f64,     // close_t / close_{t-n}
    pub rocr_prev: f64,
    pub close_now: f64,
    pub close_lag: f64,
    pub last_close: f64,
    pub rocr_label: String, // STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib ROCR100 — Rate of Change Ratio ×100 `100 · close_t / close_{t−n}`.
/// 100 = no change, >100 up, <100 down. Scales ROCR to an index-like
/// band directly comparable to CCI / PPO / ADX with zero unit-mismatch.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct Rocr100Snapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 10
    pub rocr100: f64,  // 100 · close_t / close_{t-n}
    pub rocr100_prev: f64,
    pub close_now: f64,
    pub close_lag: f64,
    pub last_close: f64,
    pub rocr100_label: String, // STRONG_UP / UP / NEUTRAL / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CORREL — rolling Pearson correlation.
/// Per-symbol instantiation: lag-1 autocorrelation of close over 30
/// bars (`ρ(close_t, close_{t-1})`). Measures serial-dependence:
/// values near +1 indicate strong momentum (consecutive closes move
/// together), near 0 a random walk, near −1 mean-reversion.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CorrelSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub correl: f64,   // Pearson correlation ∈ [-1, 1]
    pub correl_prev: f64,
    pub mean_x: f64, // mean(close_t)
    pub mean_y: f64, // mean(close_{t-1})
    pub stddev_x: f64,
    pub stddev_y: f64,
    pub last_close: f64,
    pub correl_label: String, // STRONG_MOMO / MOMO / RANDOM_WALK / MEAN_REVERT / STRONG_MEAN_REVERT / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MIN — minimum of close over a rolling window (period 30).
/// The rolling-window support level. Combined with `last_close`, the
/// distance above the minimum gives a support-proximity label.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MinSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub min_val: f64,  // min(close) over last period
    pub min_prev: f64, // min(close) ending at bar n-2
    pub max_ref: f64,  // max(close) in same window (for position pct)
    pub last_close: f64,
    pub position_pct: f64, // (close - min) / (max - min) · 100 ∈ [0, 100]
    pub min_label: String, // NEAR_LOW / MID / NEAR_HIGH / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MAX — maximum of close over a rolling window (period 30).
/// The rolling-window resistance level. Distance below the maximum gives
/// a resistance-proximity label.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MaxSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub max_val: f64,  // max(close) over last period
    pub max_prev: f64,
    pub min_ref: f64, // min(close) in same window
    pub last_close: f64,
    pub position_pct: f64, // (close - min) / (max - min) · 100
    pub max_label: String, // NEAR_HIGH / MID / NEAR_LOW / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MINMAX — both endpoints of the rolling window in one snapshot,
/// plus `range_width` and `range_pct` which expose the regime (tight
/// range = consolidation, wide range = trending).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MinMaxSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub min_val: f64,
    pub max_val: f64,
    pub range_width: f64, // max - min (price-space width)
    pub range_pct: f64,   // 100 · range_width / last_close (range-as-%-of-close)
    pub last_close: f64,
    pub position_pct: f64,    // (close - min) / range
    pub minmax_label: String, // RANGE_WIDE / RANGE_NORMAL / RANGE_TIGHT / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MININDEX — bar index at which the window minimum occurred,
/// expressed as a recency (0 = most-recent bar, period-1 = oldest).
/// Labels capture "how fresh is the low?" — a useful lagging-signal for
/// exhaustion vs. continued weakness.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MinIndexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub min_val: f64,
    pub min_index_bars_ago: usize, // 0 = most recent bar is the low; period-1 = window start
    pub min_index_bars_ago_prev: usize,
    pub last_close: f64,
    pub min_index_label: String, // FRESH_LOW / RECENT_LOW / OLD_LOW / STALE_LOW / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MAXINDEX — bar index at which the window maximum occurred,
/// expressed as a recency. Mirror of MININDEX.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MaxIndexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub max_val: f64,
    pub max_index_bars_ago: usize,
    pub max_index_bars_ago_prev: usize,
    pub last_close: f64,
    pub max_index_label: String, // FRESH_HIGH / RECENT_HIGH / OLD_HIGH / STALE_HIGH / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 70 — BBANDS / AD / ADOSC / SUM / LINEARREG_INTERCEPT ──

/// TA-Lib BBANDS — Bollinger Bands around a 20-bar SMA ± 2·σ.
/// Classic volatility-band oscillator. Position within the bands
/// (`pct_b`) and band width relative to mid (`bandwidth`) together
/// capture both where price is and how dynamic the band regime is.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BbandsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 20
    pub num_std: f64,  // 2.0
    pub upper: f64,
    pub middle: f64, // SMA_period(close)
    pub lower: f64,
    pub upper_prev: f64,
    pub middle_prev: f64,
    pub lower_prev: f64,
    pub last_close: f64,
    pub pct_b: f64,           // 100 · (close − lower) / (upper − lower)
    pub bandwidth: f64,       // 100 · (upper − lower) / middle
    pub bbands_label: String, // ABOVE_UPPER / UPPER_HALF / LOWER_HALF / BELOW_LOWER / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib AD — Chaikin Accumulation/Distribution Line.
/// Cumulative `MF × volume` where `MF = ((close − low) − (high − close)) /
/// (high − low)`. A running total of volume-weighted close bias within
/// each bar's range — rising = net buying, falling = net distribution.
/// The scalar `ad_slope` is the 10-bar linear-regression slope of the
/// line for label classification.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AdSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ad: f64,       // cumulative A/D at last bar
    pub ad_prev: f64,  // cumulative A/D at bar n-2
    pub ad_delta: f64, // ad − ad_prev
    pub ad_slope: f64, // 10-bar slope of AD series
    pub last_close: f64,
    pub ad_label: String, // STRONG_ACCUM / ACCUM / FLAT / DIST / STRONG_DIST / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib ADOSC — Chaikin Accumulation/Distribution Oscillator.
/// `fast_EMA(AD) − slow_EMA(AD)` with default (fast=3, slow=10). A
/// zero-centred momentum oscillator on the AD line — signals
/// accumulation/distribution impulses that the raw AD slope can't
/// pick up.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AdoscSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize, // 3
    pub slow_period: usize, // 10
    pub adosc: f64,
    pub adosc_prev: f64,
    pub last_close: f64,
    pub ad_ref: f64,         // underlying AD value at same bar (for cross-ref)
    pub adosc_label: String, // STRONG_BULL / BULL / FLAT / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib SUM — rolling sum of close over a 30-bar window. The raw
/// primitive SMA is built on top of (SMA = SUM / period) — distinct
/// because SUM is an absolute quantity useful for compounding
/// calculations, whereas SMA is an average. Label classifies whether
/// the sum is rising (momentum) or falling (decay) by comparing the
/// current sum to the sum one bar earlier.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct SumSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub sum: f64,
    pub sum_prev: f64,
    pub sum_delta: f64,
    pub sum_pct_change: f64, // 100 · (sum − sum_prev) / sum_prev
    pub last_close: f64,
    pub sum_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib LINEARREG_INTERCEPT — the `b` coefficient in `y = m·x + b`
/// where `y = close` and `x = bar_index` over a 14-bar window.
/// Complements the already-shipped LINEARREG (endpoint), LINEARREG_ANGLE,
/// LINEARREG_SLOPE, and TSF primitives. The intercept alone is not a
/// signal; it is the *level* the regression predicts at x=0 (the
/// oldest bar in the window). The informative scalar is
/// `intercept − last_close`, which says how far the regression
/// has walked from its oldest bar.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LinearRegInterceptSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub intercept: f64,
    pub intercept_prev: f64,
    pub slope: f64, // for cross-ref with LINEARREG_SLOPE
    pub last_close: f64,
    pub drift: f64, // last_close − intercept (how far price is from regression base)
    pub drift_pct: f64, // 100 · drift / intercept
    pub linreg_intercept_label: String, // STRONG_ADVANCE / ADVANCE / FLAT / DECLINE / STRONG_DECLINE / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 71 — AROONOSC / MINMAXINDEX / MACDEXT / MACDFIX / MAVP ──

/// TA-Lib AROONOSC — Aroon Oscillator = AROON_UP − AROON_DOWN over a
/// 14-bar window. Complements the already-shipped AROON primitive by
/// surfacing the signed differential directly — values near +100 signal
/// strong uptrend (recent high very fresh, low stale), −100 strong
/// downtrend, near zero mixed/no-trend.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct AroonoscSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 14
    pub aroonosc: f64, // aroon_up − aroon_down ∈ [-100, 100]
    pub aroonosc_prev: f64,
    pub aroon_up: f64, // for cross-ref
    pub aroon_down: f64,
    pub last_close: f64,
    pub aroonosc_label: String, // STRONG_BULL / BULL / FLAT / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MINMAXINDEX — combined rolling-window MININDEX + MAXINDEX in
/// one snapshot plus `age_diff` (bars between the two extrema) and
/// `extrema_order` (HIGH_FIRST / LOW_FIRST / SAME_BAR) which together
/// describe the window's directional signature. Completes the
/// Round 69 rolling-extrema family.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MinMaxIndexSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub period: usize, // 30
    pub min_index_bars_ago: usize,
    pub max_index_bars_ago: usize,
    pub age_diff: i64, // min_idx − max_idx (signed — positive means max is fresher)
    pub extrema_order: String, // HIGH_FIRST / LOW_FIRST / SAME_BAR
    pub last_close: f64,
    pub minmaxindex_label: String, // FRESH_HIGH / FRESH_LOW / MID / OLD_EXTREMA / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MACDEXT — MACD with *configurable MA types* for fast, slow,
/// and signal. This snapshot pins all three to SMA (the classic "simple
/// MACD" textbook form) to give agents a deterministic alternative to
/// the default EMA-based MACD. Complements the existing MACD snapshot
/// (Round 7 era) without replacing it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MacdextSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize,   // 12
    pub slow_period: usize,   // 26
    pub signal_period: usize, // 9
    pub ma_type: String,      // "SMA" for this snapshot
    pub macd: f64,            // fast_MA − slow_MA
    pub macd_prev: f64,
    pub signal: f64, // MA(macd)
    pub signal_prev: f64,
    pub hist: f64, // macd − signal
    pub hist_prev: f64,
    pub last_close: f64,
    pub macdext_label: String, // STRONG_BULL / BULL / FLAT / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MACDFIX — MACD with *hardcoded* 12/26 fast/slow (fix = fixed)
/// and configurable signal (9 default). Historically the most-widely
/// used MACD form — this snapshot surfaces the canonical 12/26/9
/// EMA-based MACD. Distinct from the existing Round 7 MACD snapshot in
/// that it exposes the hardcoded-fast/slow as an explicit constraint,
/// useful for agents wanting to verify textbook parameters.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MacdfixSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub fast_period: usize,   // 12 (fixed)
    pub slow_period: usize,   // 26 (fixed)
    pub signal_period: usize, // 9
    pub macd: f64,            // EMA12(close) − EMA26(close)
    pub macd_prev: f64,
    pub signal: f64, // EMA9(macd)
    pub signal_prev: f64,
    pub hist: f64, // macd − signal
    pub hist_prev: f64,
    pub last_close: f64,
    pub macdfix_label: String, // STRONG_BULL / BULL / FLAT / BEAR / STRONG_BEAR / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib MAVP — Moving Average with Variable Period. Unlike SMA/EMA/WMA
/// which all use a single fixed period, MAVP takes a per-bar period
/// array — the MA at bar t is computed with a t-specific lookback. This
/// snapshot uses a linear ramp (5 at start → 30 at end) to exercise the
/// polymorphic behaviour and emit a single scalar at the last bar
/// (last-bar period = 30). Label classifies sign of mavp_delta.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct MavpSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub min_period: usize,      // 5 (ramp start)
    pub max_period: usize,      // 30 (ramp end)
    pub last_bar_period: usize, // period used at final bar (== max_period)
    pub mavp: f64,
    pub mavp_prev: f64,
    pub mavp_delta: f64,
    pub last_close: f64,
    pub mavp_label: String, // STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLDOJI — Doji candlestick pattern. A doji is a single-bar
/// pattern where open ≈ close (body very small relative to range),
/// signalling indecision. TA-Lib convention: pattern_value is 100 for
/// bullish match, -100 for bearish match, 0 for no match. Doji is
/// directionally neutral by nature, so we emit 100 when present and
/// classify as NEUTRAL_PATTERN.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlDojiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // 100 if doji on last bar, 0 otherwise
    pub pattern_value_prev: i32,
    pub body_pct_range: f64, // |close-open| / (high-low) as percent
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize, // 0 if last bar matches, else bars since last match within bars_used window
    pub last_close: f64,
    pub cdl_doji_label: String, // DOJI_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLHAMMER — Hammer pattern. Single-bar bullish reversal
/// signal: small body in upper third of range, long lower shadow
/// (≥ 2× body), minimal upper shadow. TA-Lib emits 100 on match
/// (always treated as bullish in TA-Lib's reference implementation).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHammerSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // 100 if hammer on last bar, 0 otherwise
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_hammer_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLSHOOTINGSTAR — Shooting Star pattern. Mirror of hammer:
/// small body in lower third of range, long upper shadow (≥ 2× body),
/// minimal lower shadow. Bearish reversal signal. TA-Lib emits -100
/// on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlShootingStarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 if shooting star, 0 otherwise
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_shooting_star_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLENGULFING — Engulfing pattern. Two-bar reversal signal:
/// current bar's body fully engulfs prior bar's body AND the direction
/// is opposite (prior red → current green = bullish engulfing, prior
/// green → current red = bearish engulfing). TA-Lib emits 100 for
/// bullish, -100 for bearish, 0 for no match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlEngulfingSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_size_ratio: f64, // current body / prior body (>1.0 means current engulfs)
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_engulfing_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLHARAMI — Harami pattern. Two-bar reversal signal
/// (inside-bar): current bar's body fully contained within prior
/// bar's body AND direction is opposite. TA-Lib emits 100 for
/// bullish harami (prior red, current green inside), -100 for
/// bearish harami (prior green, current red inside), 0 for no match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHaramiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_size_ratio: f64, // current body / prior body (<1.0 means current contained)
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_harami_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLMORNINGSTAR — Morning Star (3-bar bullish reversal).
/// Bar 0 = large red body, bar 1 = small star (gapped or near
/// bar-0 close), bar 2 = large green body closing above bar-0
/// midpoint. Emits +100 when all three conditions hold.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlMorningStarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub penetration_pct: f64, // 100 · (bar2_close - bar0_midpoint) / bar0_body  (> 0 when bullish)
    pub star_body_pct_range: f64, // middle bar body % of range
    pub first_body_pct_range: f64,
    pub last_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_morning_star_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLEVENINGSTAR — Evening Star (3-bar bearish reversal).
/// Mirror of morning star: bar 0 = large green body, bar 1 = small
/// star, bar 2 = large red body closing below bar-0 midpoint.
/// Emits -100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlEveningStarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub penetration_pct: f64, // 100 · (bar0_midpoint - bar2_close) / bar0_body  (> 0 when bearish)
    pub star_body_pct_range: f64,
    pub first_body_pct_range: f64,
    pub last_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_evening_star_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDL3BLACKCROWS — Three Black Crows (3-bar bearish
/// continuation). Three consecutive red bars, each closing below
/// the prior close AND opening within the prior body. TA-Lib emits
/// -100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThreeBlackCrowsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub avg_body_pct_range: f64, // average body % of range across the 3 bars
    pub total_close_decline_pct: f64, // 100 · (bar2_close - bar0_open) / bar0_open
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_three_black_crows_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDL3WHITESOLDIERS — Three White Soldiers (3-bar bullish
/// continuation). Three consecutive green bars, each closing above
/// the prior close AND opening within the prior body. TA-Lib emits
/// +100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThreeWhiteSoldiersSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub avg_body_pct_range: f64,
    pub total_close_advance_pct: f64, // 100 · (bar2_close - bar0_open) / bar0_open
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_three_white_soldiers_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLDARKCLOUDCOVER — Dark Cloud Cover (2-bar bearish
/// reversal). Prior bar green with large body; current bar red,
/// opens above prior high, closes below prior midpoint (standard
/// TA-Lib penetration threshold 0.5). Emits -100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlDarkCloudCoverSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub penetration_pct: f64, // 100 · (prior_close - current_close) / prior_body
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_dark_cloud_cover_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 74 — CDLPIERCING / CDLDRAGONFLYDOJI / CDLGRAVESTONEDOJI / CDLHANGINGMAN / CDLINVERTEDHAMMER ──

/// TA-Lib CDLPIERCING — Piercing Line (2-bar bullish reversal, mirror
/// of Dark Cloud Cover). Prior bar red with large body; current bar
/// green, opens below prior low, closes above prior midpoint (≥ 50%
/// penetration). Emits +100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlPiercingSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub penetration_pct: f64, // 100 · (current_close - prior_close) / prior_body
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_piercing_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLDRAGONFLYDOJI — Dragonfly Doji (single-bar support
/// signal). Doji body (≤ 5% of range) with open ≈ high ≈ close
/// and long lower shadow. T-shape indicating rejection of lower
/// prices. TA-Lib emits +100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlDragonflyDojiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64, // % of range above body
    pub lower_shadow_pct: f64, // % of range below body (dominant for dragonfly)
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_dragonfly_doji_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLGRAVESTONEDOJI — Gravestone Doji (single-bar resistance
/// signal). Doji body (≤ 5% of range) with open ≈ low ≈ close and
/// long upper shadow. Inverted-T shape indicating rejection of
/// higher prices. TA-Lib emits -100 on match.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlGravestoneDojiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64, // dominant for gravestone
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_gravestone_doji_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLHANGINGMAN — Hanging Man (single-bar bearish reversal
/// at tops). Geometrically identical to the Hammer but appearing at
/// market tops instead of bottoms: small body in the upper third,
/// long lower shadow ≥ 2× body, minimal upper shadow. TA-Lib emits
/// -100 on match (sign-flipped from Hammer's +100 to signal bearish
/// top context vs. bullish bottom context).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHangingManSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64, // dominant (≥ 2× body)
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_hanging_man_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLINVERTEDHAMMER — Inverted Hammer (single-bar bullish
/// reversal at bottoms). Mirror of Shooting Star but appearing at
/// bottoms instead of tops: small body in the lower third, long
/// upper shadow ≥ 2× body, minimal lower shadow. TA-Lib emits
/// +100 on match (sign-flipped from Shooting Star's -100).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlInvertedHammerSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64, // dominant (≥ 2× body)
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_inverted_hammer_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLHARAMICROSS — Harami Cross (stricter 2-bar reversal).
/// Variant of Harami where the inside bar is a doji (body ≤ 5% of
/// range) rather than any small opposite-direction body. TA-Lib
/// treats this as a more potent reversal signal than regular
/// Harami; emits +100 (bullish) when prior bar is red and current
/// is a doji contained in prior body; -100 (bearish) mirror.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHaramiCrossSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64, // must be ≤ 5% to qualify as doji
    pub body_size_ratio: f64,        // cur_body / prior_body, always < 1.0 when match
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_harami_cross_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLLONGLEGGEDDOJI — Long-Legged Doji (single-bar
/// indecision with wide range). Doji body (≤ 5% of range) with
/// BOTH upper and lower shadows dominant (each ≥ 30% of range).
/// Signals strong indecision after a meaningful price excursion
/// in both directions within the bar. TA-Lib emits +100 on match
/// (treated as directionally neutral like regular doji; context
/// determines bullish/bearish implication).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlLongLeggedDojiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 pattern present, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,   // ≤ 5%
    pub upper_shadow_pct: f64, // ≥ 30%
    pub lower_shadow_pct: f64, // ≥ 30%
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_long_legged_doji_label: String, // DOJI_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLMARUBOZU — Marubozu (single-bar pure-body
/// conviction). Bar with little to no shadows (each ≤ 5% of
/// range) and body ≥ 90% of range. Bullish marubozu = green
/// (open == low, close == high), bearish marubozu = red
/// (open == high, close == low). Strongest single-bar
/// directional conviction signal. TA-Lib emits +100 (bullish)
/// or -100 (bearish).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlMarubozuSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,   // ≥ 90%
    pub upper_shadow_pct: f64, // ≤ 5%
    pub lower_shadow_pct: f64, // ≤ 5%
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_marubozu_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLSPINNINGTOP — Spinning Top (single-bar indecision
/// with moderate shadows). Small body (≤ 30% of range) centred in
/// range with BOTH upper and lower shadows larger than the body.
/// Indicates indecision but less extreme than long-legged doji.
/// TA-Lib emits +100 (green body) or -100 (red body) though both
/// are treated as indecision signals regardless of body colour.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlSpinningTopSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 green-body, -100 red-body, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,   // ≤ 30%
    pub upper_shadow_pct: f64, // > body
    pub lower_shadow_pct: f64, // > body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_spinning_top_label: String, // GREEN_BODY_PATTERN / RED_BODY_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLTRISTAR — Tri-Star (3-bar rare triple-doji
/// reversal). Three consecutive doji bars (each body ≤ 5% of
/// range). TA-Lib emits +100 (bullish) when middle doji gaps
/// below the outer two and the third doji closes above the
/// middle; -100 (bearish) mirror. Rare but high-conviction
/// reversal signal when present.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlTristarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub avg_body_pct_range: f64, // average across the three dojis
    pub middle_gap_pct: f64,     // signed % gap of middle doji from outer two
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_tristar_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLDOJISTAR — Doji Star (2-bar reversal precursor).
/// Prior bar has a real body (body ≥ 30% of range), current bar
/// is a doji (body ≤ 5% of range) that gaps away from the prior
/// close. -100 (bearish) when prior is green and current doji
/// gaps above prior close; +100 (bullish) when prior is red and
/// current doji gaps below prior close. Precursor to the full
/// 3-bar MORNINGDOJISTAR / EVENINGDOJISTAR patterns.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlDojiStarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,   // ≥ 30% to qualify as real body
    pub current_body_pct_range: f64, // ≤ 5% to qualify as doji
    pub gap_pct: f64,                // signed % gap from prior close
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_doji_star_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLMORNINGDOJISTAR — Morning Doji Star (3-bar bullish
/// reversal, doji-middle variant of R73 MORNINGSTAR). Bar 1 is a
/// long red body (≥ 30% of range); bar 2 is a doji (body ≤ 5%)
/// that gaps below bar 1's close; bar 3 is green and closes above
/// bar 1's midpoint. Stronger bullish-reversal conviction than
/// regular morning star because the doji indicates explicit
/// equilibrium after the sell-off before the recovery.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlMorningDojiStarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub bar1_body_pct_range: f64,        // ≥ 30%
    pub bar2_body_pct_range: f64,        // doji ≤ 5%
    pub bar3_close_vs_bar1_mid_pct: f64, // signed % above (positive) bar 1 midpoint
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_morning_doji_star_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLEVENINGDOJISTAR — Evening Doji Star (3-bar bearish
/// reversal, doji-middle variant of R73 EVENINGSTAR). Bar 1 is a
/// long green body; bar 2 is a doji that gaps above bar 1's close;
/// bar 3 is red and closes below bar 1's midpoint. Stronger
/// bearish-reversal conviction than regular evening star because
/// the doji indicates explicit equilibrium after the rally before
/// the breakdown.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlEveningDojiStarSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub bar1_body_pct_range: f64,        // ≥ 30%
    pub bar2_body_pct_range: f64,        // doji ≤ 5%
    pub bar3_close_vs_bar1_mid_pct: f64, // signed % below (negative) bar 1 midpoint
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_evening_doji_star_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLABANDONEDBABY — Abandoned Baby (strongest 3-bar star
/// variant). Doji "abandoned" by full-body-and-shadow gaps on both
/// sides. Bullish: bar 1 long red, bar 2 doji with bar2.high <
/// bar1.low (no overlap), bar 3 green with bar3.low > bar2.high
/// (full gap away). Bearish: mirror. Rare but very high-conviction
/// reversal signal.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlAbandonedBabySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub bar1_body_pct_range: f64, // ≥ 30%
    pub bar2_body_pct_range: f64, // doji ≤ 5%
    pub gap_down_pct: f64,        // signed % gap between bar 1 low and bar 2 high
    pub gap_up_pct: f64,          // signed % gap between bar 2 high and bar 3 low
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_abandoned_baby_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDL3INSIDE — Three Inside Up/Down (confirmed Harami).
/// Bar 1 has a long body; bar 2 is a small body of the opposite
/// colour fully contained within bar 1's body (Harami geometry);
/// bar 3 closes beyond bar 1's body in the direction opposite to
/// bar 1 (confirmation). Bullish: bar 1 red + bar 2 small green
/// inside + bar 3 closes above bar 1's open. Bearish: mirror.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThreeInsideSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub bar1_body_pct_range: f64,         // ≥ 30%
    pub body_size_ratio: f64,             // bar 2 body / bar 1 body, < 1.0 when match
    pub bar3_close_vs_bar1_open_pct: f64, // signed % distance from bar 1 open
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_three_inside_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 77 — CDLBELTHOLD / CDLCLOSINGMARUBOZU / CDLHIGHWAVE /
//    CDLLONGLINE / CDLSHORTLINE ──

/// TA-Lib CDLBELTHOLD — Belt-hold line. Long real body with virtually
/// no opening shadow. Bullish when a green candle opens at/near the low
/// of the range; bearish when a red candle opens at/near the high.
/// Strong single-bar conviction pattern. TA-Lib emits +100 / -100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlBeltHoldSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,     // long body threshold
    pub opening_shadow_pct: f64, // lower shadow for green, upper for red
    pub closing_shadow_pct: f64, // opposite-side shadow
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_belt_hold_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLCLOSINGMARUBOZU — Closing Marubozu. Long real body with
/// virtually no closing shadow. Bullish when a green candle closes at/
/// near the high; bearish when a red candle closes at/near the low.
/// TA-Lib emits +100 / -100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlClosingMarubozuSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,     // long body threshold
    pub opening_shadow_pct: f64, // lower shadow for green, upper for red
    pub closing_shadow_pct: f64, // upper shadow for green, lower for red
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_closing_marubozu_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLHIGHWAVE — High-Wave Candle. Small body but long shadows
/// on both sides, signalling strong intrabar indecision with large
/// excursion in both directions. TA-Lib emits +100 for green body,
/// -100 for red body.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHighWaveSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 green-body, -100 red-body, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,   // small body
    pub upper_shadow_pct: f64, // long upper shadow
    pub lower_shadow_pct: f64, // long lower shadow
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_high_wave_label: String, // GREEN_BODY_PATTERN / RED_BODY_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLLONGLINE — Long Line Candle. Long real body with relatively
/// small shadows at both ends. TA-Lib emits +100 for green body and
/// -100 for red body.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlLongLineSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 green-body, -100 red-body, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64, // dominant body
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_long_line_label: String, // GREEN_BODY_PATTERN / RED_BODY_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLSHORTLINE — Short Line Candle. Short real body with
/// relatively small shadows. TA-Lib emits +100 for green body and
/// -100 for red body.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlShortLineSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 green-body, -100 red-body, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64, // short body
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_short_line_label: String, // GREEN_BODY_PATTERN / RED_BODY_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 78 — CDLCOUNTERATTACK / CDLHOMINGPIGEON / CDLINNECK /
//    CDLONNECK / CDLTHRUSTING ──

/// TA-Lib CDLCOUNTERATTACK — Counterattack lines. Two opposite-colour
/// long candles with a gap open in the direction of the prior bar, then
/// a close back at/near the prior close. Bullish when the first bar is
/// red and the second green; bearish mirror for green→red.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlCounterattackSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub gap_open_pct: f64,        // positive in the direction of the gap
    pub close_diff_pct_body: f64, // abs close-vs-prior-close difference as % of prior body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_counterattack_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLHOMINGPIGEON — Homing Pigeon. A bearish harami variant:
/// long red body followed by a smaller red body fully inside the first.
/// Bullish reversal pattern in TA-Lib, emitting +100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHomingPigeonSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub body_size_ratio: f64,       // current / prior
    pub inner_body_margin_pct: f64, // min inner-body clearance as % of prior body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_homing_pigeon_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLINNECK — In-Neck pattern. Long red body followed by a
/// green candle that gaps below the prior low and closes slightly into
/// the prior real body. Bearish continuation, TA-Lib emits -100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlInNeckSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub gap_open_pct: f64,
    pub penetration_pct: f64, // close into prior body as % of prior body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_in_neck_label: String, // BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLONNECK — On-Neck pattern. Long red body followed by a
/// green candle that gaps below the prior low and closes back at/near
/// the prior close. Bearish continuation, TA-Lib emits -100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlOnNeckSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub gap_open_pct: f64,
    pub close_match_pct: f64, // abs close-vs-prior-close difference as % of prior body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_on_neck_label: String, // BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLTHRUSTING — Thrusting pattern. Long red body followed by a
/// green candle that gaps below the prior low and closes into the prior
/// body, but not as deep as the midpoint. Bearish continuation, emits
/// -100.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThrustingSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub gap_open_pct: f64,
    pub penetration_pct: f64, // deeper than in-neck but below midpoint
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_thrusting_label: String, // BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 79 — CDL2CROWS / CDL3LINESTRIKE / CDL3OUTSIDE /
//    CDLMATCHINGLOW ──

/// TA-Lib CDL2CROWS — Two Crows. Long green body, then a gap-up red
/// candle, followed by another red candle that opens inside the second
/// body and closes back inside the first real body. Bearish reversal.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlTwoCrowsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_gap_pct: f64,        // gap-up of bar 2 body vs bar 1 body
    pub third_penetration_pct: f64, // bar 3 close into bar 1 body as % of bar 1 body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_two_crows_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDL3LINESTRIKE — Three Line Strike. Three same-direction
/// candles followed by a large opposite-colour strike candle that
/// closes beyond the first bar's open. Reversal signal.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThreeLineStrikeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub avg_first_three_body_pct_range: f64,
    pub strike_body_pct_range: f64,
    pub strike_close_vs_first_open_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_three_line_strike_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDL3OUTSIDE — Three Outside Up/Down. An engulfing reversal
/// confirmed by a third candle continuing in the same direction.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThreeOutsideSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub engulf_body_ratio: f64,      // bar2 body / bar1 body
    pub confirmation_pct_body2: f64, // bar3 close extension beyond bar2 close as % of bar2 body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_three_outside_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLMATCHINGLOW — Matching Low. Two red candles that close at
/// nearly the same level, signalling potential support. Bullish in
/// TA-Lib (+100).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlMatchingLowSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub close_match_pct_body: f64, // abs(close2-close1) as % of first body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_matching_low_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 80 — CDLSEPARATINGLINES / CDLSTICKSANDWICH /
//    CDLRICKSHAWMAN / CDLTAKURI ──

/// TA-Lib CDLSEPARATINGLINES — Separating Lines. Opposite-colour
/// candles with the same open, where the second resumes the prevailing
/// direction. Continuation pattern with both bullish and bearish forms.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlSeparatingLinesSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub prior_body_pct_range: f64,
    pub current_body_pct_range: f64,
    pub open_match_pct_body: f64, // abs(open2-open1) as % of first body
    pub continuation_pct_body: f64, // close extension beyond first open as % of first body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_separating_lines_label: String, // BULLISH_CONTINUATION / BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLSTICKSANDWICH — Stick Sandwich. Red / green / red where
/// the first and third closes match, marking potential support.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlStickSandwichSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub close_match_pct_body: f64,
    pub middle_rebound_pct: f64, // middle close above first close as % of first body
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_stick_sandwich_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLRICKSHAWMAN — Rickshaw Man. A centered doji with long
/// upper and lower shadows. Neutral indecision pattern, reported here
/// as +100 when present for parity/discoverability.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlRickshawManSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 present, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub body_midpoint_offset_pct: f64, // distance of body midpoint from range midpoint
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_rickshaw_man_label: String, // RICKSHAW_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLTAKURI — Takuri. Dragonfly-like doji with an especially
/// long lower shadow. Bullish reversal variant.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlTakuriSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub body_pct_range: f64,
    pub upper_shadow_pct: f64,
    pub lower_shadow_pct: f64,
    pub lower_to_upper_ratio: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_takuri_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 81/82 — harder TA-Lib CDL parity pack ──────────────────

/// TA-Lib CDL3STARSINSOUTH — Three Stars in the South. A 3-bar bullish
/// reversal made of three descending red candles where downside pressure
/// progressively contracts: a long black candle with a long lower shadow,
/// then a smaller black candle with a higher low, then a small black bar
/// nested inside the second bar.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlThreeStarsInSouthSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub first_lower_shadow_pct: f64,
    pub second_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub third_inside_pct_range: f64, // how deeply bar 3 sits inside bar 2 range
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_three_stars_in_south_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLIDENTICAL3CROWS — Identical Three Crows. Bearish 3-bar
/// continuation: three long red candles with each new candle opening
/// near the prior close and extending the decline.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlIdenticalThreeCrowsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub avg_body_pct_range: f64,
    pub open1_vs_close0_pct_body: f64, // abs(open2-close1) as % of body1
    pub open2_vs_close1_pct_body: f64, // abs(open3-close2) as % of body2
    pub total_close_decline_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_identical_three_crows_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLKICKING — Kicking. Two opposite-colour marubozu candles
/// separated by a clean gap between their full ranges.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlKickingSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub gap_pct_range: f64, // gap magnitude as % of first bar range
    pub second_to_first_body_ratio: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_kicking_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLKICKINGBYLENGTH — Kicking where bull/bear direction is
/// assigned from the longer of the two marubozu bodies.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlKickingByLengthSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub gap_pct_range: f64,
    pub dominant_body_ratio: f64, // larger body / smaller body
    pub dominant_side: String,    // FIRST_BAR / SECOND_BAR / NONE
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_kicking_by_length_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLLADDERBOTTOM — Ladder Bottom. Five-bar bullish reversal:
/// three descending red candles, a fourth red "rung" with an upper
/// shadow, then a strong green breakout candle.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlLadderBottomSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub avg_first_three_body_pct_range: f64,
    pub fourth_body_pct_range: f64,
    pub fourth_upper_shadow_pct: f64,
    pub fifth_body_pct_range: f64,
    pub breakout_pct_vs_fourth_high: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_ladder_bottom_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLUNIQUE3RIVER — Unique 3 River. Three-bar bullish reversal:
/// long red candle, then a smaller red candle with an extended lower
/// shadow, followed by a small green candle tucked inside the second.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlUniqueThreeRiverSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub second_lower_shadow_pct: f64,
    pub third_body_pct_range: f64,
    pub third_close_vs_second_close_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_unique_three_river_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 83/84 — next TA-Lib CDL parity pack ────────────────────

/// TA-Lib CDLADVANCEBLOCK — Advance Block. Three rising green candles
/// whose progress weakens as bodies shrink and upper shadows lengthen.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlAdvanceBlockSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub third_upper_shadow_pct: f64,
    pub total_close_gain_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_advance_block_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLBREAKAWAY — Breakaway. Five-bar reversal pattern with an
/// initial gap in trend direction and a final candle that closes back
/// into that gap.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlBreakawaySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub initial_gap_pct_range: f64,
    pub fifth_body_pct_range: f64,
    pub gap_retracement_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_breakaway_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLGAPSIDESIDEWHITE — Up/Down-gap side-by-side white lines.
/// Two similar green candles that hold a gap versus the prior candle.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlGapSideSideWhiteSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish continuation, -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub gap_pct_range: f64,
    pub second_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub open_similarity_pct_body: f64,
    pub close_similarity_pct_body: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_gap_side_side_white_label: String, // BULLISH_CONTINUATION / BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLUPSIDEGAP2CROWS — Upside Gap Two Crows. Long green candle,
/// gap-up red candle, then another red candle that opens higher and
/// closes back into the gap.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlUpsideGapTwoCrowsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub upside_gap_pct_range: f64,
    pub third_open_above_second_pct_body: f64,
    pub third_close_into_gap_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_upside_gap_two_crows_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLXSIDEGAP3METHODS — Upside/Downside Gap Three Methods.
/// Two same-direction candles gap away from the first, then an
/// opposite-colour candle closes into that gap without fully reversing it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlXSideGapThreeMethodsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish continuation, -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub gap_pct_range: f64,
    pub second_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub gap_fill_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_xside_gap_three_methods_label: String, // BULLISH_CONTINUATION / BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLCONCEALBABYSWALL — Concealing Baby Swallow. Four black
/// candles where the first two are marubozu-like, the third gaps down,
/// and the fourth engulfs the third candle's range.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlConcealBabySwallowSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish reversal, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub third_upper_shadow_pct: f64,
    pub fourth_range_engulf_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_conceal_baby_swallow_label: String, // BULLISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 85/86 — stateful TA-Lib CDL parity pack ────────────────

/// TA-Lib CDLHIKKAKE — Hikkake. Inside-bar setup followed by a false
/// break to one side of the inside bar.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHikkakeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub inside_width_pct_mother: f64,
    pub false_break_extension_pct: f64,
    pub trigger_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_hikkake_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLHIKKAKEMOD — Modified Hikkake. Hikkake-like inside-bar
/// trap followed by an explicit confirmation bar.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlHikkakeModSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish, -100 bearish, 0 none
    pub pattern_value_prev: i32,
    pub inside_width_pct_mother: f64,
    pub false_break_extension_pct: f64,
    pub confirmation_extension_pct: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_hikkake_mod_label: String, // BULLISH_PATTERN / BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLMATHOLD — Mat Hold. A strong trend candle, a gapped pause,
/// two holding candles, then a continuation breakout.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlMatHoldSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish continuation, -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub middle_avg_body_pct_range: f64,
    pub initial_gap_pct_range: f64,
    pub hold_depth_pct_body: f64,
    pub final_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_mat_hold_label: String, // BULLISH_CONTINUATION / BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLRISEFALL3METHODS — Rising/Falling Three Methods. Long
/// trend candle, three small counter-trend candles inside it, then a
/// continuation candle in the original direction.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlRiseFallThreeMethodsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish continuation, -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub middle_avg_body_pct_range: f64,
    pub containment_pct_body: f64,
    pub final_body_pct_range: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_rise_fall_three_methods_label: String, // BULLISH_CONTINUATION / BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 87/88 — remaining TA-Lib CDL parity pack ───────────────

/// TA-Lib CDLSTALLEDPATTERN — Stalled Pattern. Three advancing white
/// candles where the third gaps up but loses momentum with a small real
/// body and meaningful upper shadow.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlStalledPatternSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // -100 bearish reversal, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub third_open_gap_pct_range: f64,
    pub third_upper_shadow_pct: f64,
    pub close_progress_pct_prev_leg: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_stalled_pattern_label: String, // BEARISH_PATTERN / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

/// TA-Lib CDLTASUKIGAP — Tasuki Gap. Two same-direction candles with a
/// trend gap, followed by an opposite-colour candle that retraces into
/// the gap without fully closing it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct CdlTasukiGapSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pattern_value: i32, // +100 bullish continuation, -100 bearish continuation, 0 none
    pub pattern_value_prev: i32,
    pub first_body_pct_range: f64,
    pub second_body_pct_range: f64,
    pub third_body_pct_range: f64,
    pub gap_pct_range: f64,
    pub gap_fill_pct: f64,
    pub third_open_pct_second_body: f64,
    pub last_bar_match: bool,
    pub days_since_pattern: usize,
    pub last_close: f64,
    pub cdl_tasuki_gap_label: String, // BULLISH_CONTINUATION / BEARISH_CONTINUATION / NO_PATTERN / INSUFFICIENT_DATA
    pub note: String,
}

// ── Round 76 (Quant Stats) surfaces ───────────────────────────────

/// MODSHARPE — Pezier-White Adjusted Sharpe Ratio.
/// Classical Sharpe SR = √252 · E[r]/σ[r] assumes normal returns. The
/// Pezier-White (2006) adjustment corrects for higher moments:
///     ASR = SR · [1 + (S/6)·SR − ((K−3)/24)·SR²]
/// where S is skewness and K is kurtosis of bar-level returns. For
/// negatively-skewed fat-tailed distributions the adjustment reduces
/// the headline Sharpe; for positively-skewed returns it can boost it.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ModSharpeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub annualization_factor: f64, // 252 for daily bars
    pub mean_return_bar: f64,      // arithmetic mean of bar returns
    pub stdev_return_bar: f64,
    pub skewness: f64,           // 3rd standardised central moment
    pub excess_kurtosis: f64,    // K − 3
    pub sharpe_ratio: f64,       // annualised classical Sharpe
    pub adjusted_sharpe: f64,    // annualised Pezier-White ASR
    pub adjustment_factor: f64,  // ASR / SR
    pub modsharpe_label: String, // STRONG_POS / MODERATE_POS / WEAK / MODERATE_NEG / STRONG_NEG / INSUFFICIENT_DATA
    pub note: String,
}

/// HSIEHTEST — Hsieh (1989) third-moment nonlinearity test.
/// Fits AR(1) residuals ε_t = r_t − μ − φ·r_{t-1}, then probes the
/// standardised third cross-moment T(i,j) = E[ε_{t−i} ε_{t−j} ε_t]/σ³.
/// Under linearity, T(i,j) = 0 for all (i,j). We test lags (1,1) and
/// (2,2); |z| > 1.96 indicates statistically detectable nonlinearity.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HsiehTestSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub ar_order: usize, // 1
    pub t_11: f64,       // standardised third moment at lag (1,1)
    pub t_22: f64,       // at (2,2)
    pub z_11: f64,       // asymptotic z-stat of T(1,1) · √n / √6
    pub z_22: f64,
    pub max_abs_z: f64,      // max(|z_11|, |z_22|)
    pub critical_95: f64,    // 1.96
    pub reject_null: bool,   // max_abs_z > 1.96
    pub hsieh_label: String, // LINEAR / MILD_NONLIN / STRONG_NONLIN / INSUFFICIENT_DATA
    pub note: String,
}

/// CHOWBREAK — Chow (1960) mean-shift structural break F-test.
/// Splits the return series at n/2 and compares the pooled-mean
/// RSS to the sum of within-group RSS. F = [(RSS_p − RSS_u)/k] /
/// [RSS_u/(n−2k)] with k=1 regressor (constant). Large F ⇒ reject
/// "no break at n/2". Useful as a quick structural-change screen.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct ChowBreakSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub break_point_idx: usize, // n/2
    pub rss_pooled: f64,
    pub rss_unrestricted: f64, // RSS_1 + RSS_2 with separate means
    pub mean_pre: f64,
    pub mean_post: f64,
    pub k_regressors: usize, // 1 (constant-only model)
    pub f_stat: f64,
    pub df_num: usize,
    pub df_den: usize,
    pub critical_95: f64, // ≈ 3.84 for k=1, large n
    pub reject_null: bool,
    pub chowbreak_label: String, // NO_BREAK / MILD_BREAK / STRONG_BREAK / INSUFFICIENT_DATA
    pub note: String,
}

/// DRIFTBURST — Christensen-Oomen-Renò (2018) drift-burst hypothesis test.
/// Scans the return series with a Gaussian kernel to compute a
/// rolling drift-to-volatility ratio T(t) = √h · μ̂(t)/σ̂(t). Large
/// |T(t)| is a local "drift burst" — a period where the trend
/// dominates the volatility scale. Reports the maximum over the
/// window and the number of excursions above |T|>3 (approx 99%
/// pointwise critical value).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DriftBurstSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub kernel_bandwidth_bars: f64, // σ of Gaussian kernel (half-width)
    pub max_abs_statistic: f64,     // max_t |T(t)|
    pub max_stat_signed: f64,       // signed T at argmax |T|
    pub max_at_offset: usize,       // bars before series end (0 = latest)
    pub excursions_gt_3: usize,     // count of t with |T(t)| > 3
    pub critical_99_approx: f64,    // 3.0 (pointwise)
    pub driftburst_label: String,   // NO_BURST / MILD_BURST / STRONG_BURST / INSUFFICIENT_DATA
    pub note: String,
}

/// HLVCLUST — Parkinson high-low volatility clustering (Ljung-Box on
/// log-range series). The Parkinson range estimator is
/// σ̂²_P(t) = (1/(4 ln 2)) · ln(H_t/L_t)². We form v_t = ln(σ̂_P(t))
/// (or equivalently 0.5·ln(ln(H/L)²) up to a constant) and apply
/// Ljung-Box to lag h=10. Rejecting white noise on v_t confirms
/// volatility clustering even without return-based GARCH machinery.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct HlvClustSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub lag_h: usize,                  // 10
    pub parkinson_vol_bar: f64,        // mean σ̂_P per bar
    pub parkinson_vol_annualised: f64, // × √252
    pub ac_lag1: f64,                  // lag-1 autocorrelation of log-range series
    pub ac_lag5: f64,                  // lag-5
    pub lb_q_stat: f64,                // Ljung-Box Q at lag h
    pub critical_95: f64,              // χ²(h) 95%
    pub p_value: f64,
    pub reject_null: bool,
    pub hlvclust_label: String, // NO_CLUST / MILD_CLUST / STRONG_CLUST / INSUFFICIENT_DATA
    pub note: String,
}

/// YANGZHANG — Yang-Zhang (2000) three-component range volatility estimator.
/// σ²_YZ = σ²_O + k·σ²_C + (1-k)·σ²_RS, where σ²_O is overnight open-vs-prev-close
/// variance, σ²_C is close-to-close variance, σ²_RS is the Rogers-Satchell intraday
/// component, and k = 0.34 / (1.34 + (n+1)/(n-1)) minimises variance under a
/// drift-free Brownian assumption. Asymptotically the most efficient of the
/// range-based estimators that use OHLC data.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct YangZhangVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub overnight_var: f64,         // σ²_O
    pub open_to_close_var: f64,     // σ²_C (open-to-close close variant)
    pub rs_component: f64,          // σ²_RS
    pub k_weight: f64,              // 0.34 / (1.34 + (n+1)/(n-1))
    pub yz_vol_bar: f64,            // √σ²_YZ per bar
    pub yz_vol_annualised_pct: f64, // yz_vol_bar × √252 × 100
    pub cc_vol_annualised_pct: f64, // close-to-close comparison σ × √252 × 100
    pub efficiency_vs_close: f64,   // cc_vol / yz_vol (higher = YZ more efficient)
    pub yangzhang_label: String, // VERY_LOW / LOW / MODERATE / HIGH / VERY_HIGH / INSUFFICIENT_DATA
    pub note: String,
}

/// KUIPER — Kuiper (1960) two-sided empirical CDF goodness-of-fit statistic
/// against standard normal. V = D⁺ + D⁻ where D⁺ = max(F_n(x) − F(x)) and
/// D⁻ = max(F(x) − F_n(x)). More sensitive to tail departures than
/// Kolmogorov-Smirnov. Uses Stephens (1970) finite-n modification
/// V* = V · (√n + 0.155 + 0.24/√n); reject normality at 95% if V* > 1.747.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KuiperSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mean: f64,            // sample μ of returns (for standardisation)
    pub stdev: f64,           // sample σ
    pub d_plus: f64,          // max(F_n − F)
    pub d_minus: f64,         // max(F − F_n)
    pub v_stat: f64,          // D⁺ + D⁻
    pub v_stat_adj: f64,      // Stephens-modified V*
    pub critical_95: f64,     // 1.747 for standard normal
    pub p_value_approx: f64,  // Stephens 1970 approximation
    pub reject_null: bool,    // V* > 1.747
    pub kuiper_label: String, // NORMAL / MILD_DEPART / STRONG_DEPART / INSUFFICIENT_DATA
    pub note: String,
}

/// DAGOSTINO — D'Agostino-Pearson (1973) K² omnibus normality test.
/// Transforms sample skewness via D'Agostino (1970) to z_skew and
/// sample kurtosis via Anscombe-Glynn (1983) to z_kurt; combined
/// K² = z_skew² + z_kurt² is asymptotically χ²(2) under H0: normal.
/// Complements Jarque-Bera by exposing whether skew or kurt dominates
/// the departure. Reject at 95% if K² > 5.991.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct DagostinoSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub skewness: f64,        // sample b1
    pub excess_kurtosis: f64, // sample b2 − 3
    pub z_skew: f64,          // D'Agostino transformed skew z-stat
    pub z_kurt: f64,          // Anscombe-Glynn transformed kurt z-stat
    pub k2_stat: f64,         // z_skew² + z_kurt²
    pub critical_95: f64,     // 5.991
    pub p_value: f64,
    pub reject_null: bool,
    pub dagostino_label: String, // NORMAL / SKEW_DOMINANT / KURT_DOMINANT / BOTH_DEPART / INSUFFICIENT_DATA
    pub note: String,
}

/// BAIPERRON — Bai-Perron (1998) sup-F structural-break test with
/// interior search over [0.15n, 0.85n] (Andrews 1993 trimming).
/// Extends CHOWBREAK by searching *where* the break is rather than
/// assuming n/2. Reports sup-F over the trimmed interior and the
/// argmax break index. Rejects H0 "no break" at 95% if sup-F exceeds
/// the Andrews (1993) critical value (≈8.58 for 15% trim, k=1).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct BaiPerronSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub trim_fraction: f64,    // 0.15
    pub search_lo: usize,      // ⌈0.15n⌉
    pub search_hi: usize,      // ⌊0.85n⌋
    pub best_break_idx: usize, // argmax over search range
    pub sup_f_stat: f64,
    pub mean_pre: f64,
    pub mean_post: f64,
    pub rss_no_break: f64,
    pub rss_at_best: f64,
    pub critical_95: f64,    // Andrews 1993: ≈8.58 for 15% trim, k=1
    pub p_value_approx: f64, // Hansen (1997) F-asymptotic approx
    pub reject_null: bool,
    pub baiperron_label: String, // NO_BREAK / MILD_BREAK / STRONG_BREAK / INSUFFICIENT_DATA
    pub note: String,
}

/// KUPIECPOF — Kupiec (1995) Proportion-of-Failures VaR backtest.
/// Builds a rolling historical-VaR_{α=0.95} from the first `rolling_window`
/// bars and counts exceedances in the remaining test window. Likelihood
/// ratio: LR_POF = −2·ln[((1−α)^{T_ok}·α^{T_fail}) / ((1−p̂)^{T_ok}·p̂^{T_fail})]
/// where p̂ is the realised exceedance rate. LR_POF is asymptotically χ²(1)
/// under H0: realised exceedance rate equals nominal α.
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct KupiecPofSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub confidence_level: f64,        // 0.95
    pub nominal_exceedance_rate: f64, // 1 − confidence_level = 0.05
    pub rolling_window: usize,        // 60
    pub test_window: usize,           // bars_used − rolling_window
    pub var_latest_bar: f64, // latest VaR estimate (positive number; -VaR is the threshold)
    pub n_exceedances: usize,
    pub expected_exceedances: f64,
    pub realised_exceedance_rate: f64,
    pub lr_pof_stat: f64,
    pub critical_95: f64, // 3.841 = χ²_95(1)
    pub p_value: f64,
    pub reject_null: bool,
    pub kupiec_label: String, // GOOD_FIT / OVER_ESTIMATED / UNDER_ESTIMATED / INSUFFICIENT_DATA
    pub note: String,
}
