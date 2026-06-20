use serde::{Deserialize, Serialize};

// Momentum, liquidity, breakout, cash-cycle, and credit research types

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
/// Fuses cached ALTZ + PTFS + LEV + ACRL snapshots from .
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
