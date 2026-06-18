use serde::{Deserialize, Serialize};

// Value, quality, risk, insider-streak, and coverage factor research types

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
