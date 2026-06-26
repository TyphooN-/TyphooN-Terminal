use serde::{Deserialize, Serialize};

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

// World index, market mover, sector performance, and WACC research types
