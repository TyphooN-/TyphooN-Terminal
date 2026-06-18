use serde::{Deserialize, Serialize};

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
