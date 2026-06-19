use serde::{Deserialize, Serialize};

// Value/quality/risk ranks, relative EPS growth, and PEAD event research types

/// VRK — Value Rank vs sector peers snapshot.
/// Percentile rank of `ValueSnapshot.composite_score` within the same sector.
/// Higher percentile = better value (label ladder matches VAL's "DEEP_VALUE is good").
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ValueRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub composite_score: f64,    // subject's VAL composite (copied)
    pub peers_considered: usize, // peers in the same sector with a VAL snapshot
    pub peers_with_data: usize,  // same as peers_considered today
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64, // 0..100 (higher = better value)
    pub rank_position: usize, // 1-based (1 = best value in cohort)
    pub rank_label: String, // "TOP_DECILE" | "TOP_QUARTILE" | "ABOVE_MEDIAN" | "BELOW_MEDIAN" | "BOTTOM_QUARTILE" | "BOTTOM_DECILE" | "NO_DATA"
    pub note: String,
}

/// QRK — Quality Rank vs sector peers snapshot.
/// Percentile rank of `QualitySnapshot.composite_score` within the same sector.
/// Higher percentile = higher quality.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct QualityRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub composite_score: f64,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64, // 0..100 (higher = better quality)
    pub rank_position: usize,
    pub rank_label: String, // same ladder as VRK
    pub note: String,
}

/// RRK — Risk Rank vs sector peers snapshot.
/// Percentile rank of `RiskSnapshot.composite_score` within the same sector.
/// RISK composite is higher = riskier, so this snapshot *inverts* the percentile:
/// higher `percentile_rank` here = SAFER than peers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RiskRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub composite_score: f64, // subject's RISK composite (higher = riskier)
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_score: f64,
    pub sector_p25: f64,
    pub sector_p75: f64,
    pub percentile_rank: f64, // 0..100 (higher = SAFER vs peers)
    pub rank_position: usize, // 1-based (1 = safest in cohort)
    pub rank_label: String, // "SAFEST_DECILE" | "SAFEST_QUARTILE" | "ABOVE_MEDIAN_SAFE" | "BELOW_MEDIAN_RISKY" | "BOTTOM_QUARTILE_RISKY" | "RISKIEST_DECILE" | "NO_DATA"
    pub note: String,
}

/// RELEPSGR — Relative 3y EPS CAGR vs sector median snapshot.
/// CAGR computed over `FinancialStatements.income_annual[].eps` when at least
/// 4 annual rows exist (latest vs latest-3y = 3-year CAGR).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RelativeEpsGrowthSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub latest_eps: f64,
    pub earliest_eps: f64,
    pub years_used: usize,
    pub symbol_cagr_pct: f64,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_cagr_pct: f64,
    pub sector_p25_cagr_pct: f64,
    pub sector_p75_cagr_pct: f64,
    pub gap_to_median_pp: f64, // symbol_cagr - sector_median (in percentage points)
    pub relative_label: String, // "FAR_ABOVE" | "ABOVE" | "INLINE" | "BELOW" | "FAR_BELOW" | "CAGR_NEGATIVE" | "NO_DATA"
    pub note: String,
}

/// PEAD — Per-event drift row (one per earnings announcement within the window).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PeadEventRow {
    pub event_date: String,
    pub surprise_pct: f64,
    pub classification: String, // "BEAT" | "MISS" | "INLINE"
    pub drift_1d_pct: f64,
    pub drift_3d_pct: f64,
    pub drift_5d_pct: f64,
    pub drift_10d_pct: f64,
}

/// PEAD — Post-Earnings-Announcement Drift snapshot.
/// Joins cached `EarningsSurprise` rows with cached `HistoricalPriceRow` bars
/// to measure average forward drift over 1 / 3 / 5 / 10 trading days after
/// each announcement.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PeadSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub num_events: usize,  // surprises in the cache
    pub events_used: usize, // surprises successfully matched to HP bars
    pub avg_drift_1d_pct: f64,
    pub avg_drift_3d_pct: f64,
    pub avg_drift_5d_pct: f64,
    pub avg_drift_10d_pct: f64,
    pub beat_event_drift_5d_pct: f64,
    pub miss_event_drift_5d_pct: f64,
    pub latest_event_date: String,
    pub latest_event_surprise_pct: f64,
    pub latest_event_drift_5d_pct: f64,
    pub drift_direction_label: String, // "DRIFT_UP" | "DRIFT_DOWN" | "MIXED" | "INSUFFICIENT_DATA"
    pub rows: Vec<PeadEventRow>,
    pub note: String,
}
