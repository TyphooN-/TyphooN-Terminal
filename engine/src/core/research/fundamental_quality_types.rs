use serde::{Deserialize, Serialize};

// Fundamental quality, solvency, volatility-estimator, EPS beat, and price-target-dispersion research types

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
