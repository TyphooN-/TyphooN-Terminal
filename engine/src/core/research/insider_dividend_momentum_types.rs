use serde::{Deserialize, Serialize};

// Insider activity, dividend growth, earnings revision, sector rotation, and upgrade/downgrade momentum research types

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
