use serde::{Deserialize, Serialize};

// Dividend, earnings-estimate, rating-change, and treasury-yield research types

/// DVD — single historical dividend payment.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DividendRecord {
    pub ex_date: String, // YYYY-MM-DD
    pub pay_date: String,
    pub record_date: String,
    pub declaration_date: String,
    pub amount: f64,          // cash per share
    pub adjusted_amount: f64, // split-adjusted
    pub label: String,        // e.g. "Regular Cash"
}

/// EEB — one forward earnings estimate row (one fiscal period).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EarningsEstimate {
    pub date: String, // period end YYYY-MM-DD
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
    pub date: String, // YYYY-MM-DD
    pub symbol: String,
    pub company: String,
    pub firm: String,   // publisher / analyst house
    pub action: String, // "upgrade" | "downgrade" | "initiation" | "maintain"
    pub from_grade: String,
    pub to_grade: String,
    pub price_target: f64,
}

/// GY — US Treasury yield curve snapshot row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TreasuryYield {
    pub tenor: String,  // "13W" | "5Y" | "10Y" | "30Y"
    pub ticker: String, // Yahoo ticker ^IRX etc
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
