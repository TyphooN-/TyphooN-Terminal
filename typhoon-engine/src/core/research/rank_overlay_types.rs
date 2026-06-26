use super::FactorComponent;
use serde::{Deserialize, Serialize};

// Size, momentum, drift, operating-quality, revenue-growth, sentiment, ownership, and surprise-rank research types

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
/// One-layer composite over raw cached research surfaces (PTFS, MARGINS, ACRL),
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

// ── rank overlays + surprise streak ────────────────────

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

// ── dividend/earnings/rating rank overlays + gap/streak ─

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
