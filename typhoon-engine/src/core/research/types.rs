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
    /// Total employees (Finnhub `employeeTotal`); 0.0 = unknown (ADR-116).
    #[serde(default)]
    pub employees: f64,
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

// Additional corporate action and market reference types

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
