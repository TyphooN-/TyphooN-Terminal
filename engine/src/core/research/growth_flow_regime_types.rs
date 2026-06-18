use serde::{Deserialize, Serialize};

// GARP growth, flow, market-regime, relative-volume, and margin research types

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
