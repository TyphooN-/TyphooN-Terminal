use super::DivgAnnualRow;
use serde::{Deserialize, Serialize};

// Yield, short-interest, historical-volatility, drawdown, return-distribution, and behavior-stat research types

/// DVDYIELDRANK — Dividend Yield Rank vs Sector Peers.
/// Percentile rank of `Fundamentals.dividend_yield` within the same sector.
/// Non-payers (`dividend_yield.is_none() || dividend_yield == 0.0`) are
/// filtered out so the cohort captures dividend-paying names only.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DividendYieldRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub dividend_yield_pct: f64, // subject's current dividend yield %
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_yield_pct: f64,
    pub sector_p25_yield_pct: f64,
    pub sector_p75_yield_pct: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String, // standard decile ladder
    pub note: String,
}

/// SHRANK — Short Interest Rank vs Sector Peers.
/// Percentile rank of `Fundamentals.short_percent_of_float` within the same
/// sector, risk-inverted so a *lower* short interest earns a *higher* (safer)
/// rank. Names with no short interest data are filtered out.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShortInterestRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub short_pct_of_float: f64,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_short_pct: f64,
    pub sector_p25_short_pct: f64,
    pub sector_p75_short_pct: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String, // risk-inverted: SAFEST_DECILE (lowest short) → RISKIEST_DECILE
    pub note: String,
}

/// ATRANN — Annualized ATR (Volatility Regime).
/// Pure symbol-local time-series stat over the cached HP daily bars. Computes
/// the 14-period Average True Range (Wilder) on the most recent 253 sessions,
/// annualizes via √252, and maps to a volatility regime label.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AnnualizedAtrSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize, // sessions in the window (<=253)
    pub latest_close: f64,
    pub atr14: f64,              // 14-period Wilder ATR in price units
    pub atr14_pct: f64,          // atr14 / latest_close × 100
    pub atr_annualized_pct: f64, // atr14_pct × √252
    pub regime_label: String, // "LOW_VOL" | "NORMAL_VOL" | "HIGH_VOL" | "EXTREME_VOL" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// DDHIST — Drawdown History.
/// Pure symbol-local time-series stat over the same HP window. Tracks the
/// maximum drawdown (deepest peak-to-trough decline), the longest drawdown
/// duration (sessions from peak to recovery), the number of 5% corrections
/// (local peaks followed by 5%+ declines), and the current drawdown from the
/// running peak.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DrawdownHistorySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub max_drawdown_pct: f64, // deepest drawdown in the window (negative)
    pub max_drawdown_peak_date: String,
    pub max_drawdown_trough_date: String,
    pub longest_drawdown_days: usize, // sessions from peak to recovery (or to end of window if unrecovered)
    pub corrections_5pct: usize,      // count of local-peak-to-trough declines ≥5%
    pub corrections_10pct: usize,     // count of local-peak-to-trough declines ≥10%
    pub current_drawdown_pct: f64,    // latest close vs running peak (negative or 0)
    pub regime_label: String, // "RECOVERING" | "SHALLOW" | "MEANINGFUL" | "SEVERE" | "CATASTROPHIC" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// PRICEPERF — Multi-horizon Price Performance.
/// Pure symbol-local time-series stat over the HP cache. Computes total
/// returns at 1M (21 sessions), 3M (63), 6M (126), YTD (since Jan 1 of
/// as_of's year), and 1Y (253) lookbacks.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PricePerformanceSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub latest_close: f64,
    pub ret_1m_pct: f64, // % change over trailing 21 sessions
    pub ret_3m_pct: f64,
    pub ret_6m_pct: f64,
    pub ret_ytd_pct: f64,    // % change from first session of as_of's year
    pub ret_1y_pct: f64,     // % change over trailing 253 sessions
    pub trend_label: String, // "STRONG_BULL" | "BULL" | "NEUTRAL" | "BEAR" | "STRONG_BEAR" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── deferred benchmark / peer-relative parity ──

/// MOMRANK_MULTI — sector-relative percentile rank of cached PRICEPERF
/// horizons. Higher recent returns vs peers earn a higher rank, with a
/// weighted composite across 1M / 3M / 6M / YTD / 1Y returns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MomentumRankMultiSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub ret_1m_pct: f64,
    pub ret_3m_pct: f64,
    pub ret_6m_pct: f64,
    pub ret_ytd_pct: f64,
    pub ret_1y_pct: f64,
    pub pct_1m: f64,
    pub pct_3m: f64,
    pub pct_6m: f64,
    pub pct_ytd: f64,
    pub pct_1y: f64,
    pub composite_percentile: f64, // weighted blend of horizon percentiles
    pub horizons_above_median: usize, // 0..=5
    pub rank_position: usize,      // 1 = strongest momentum in sector
    pub rank_label: String,        // TOP_DECILE .. BOTTOM_DECILE | INSUFFICIENT_DATA | NO_DATA
    pub note: String,
}

/// CORRSTK — rolling benchmark correlation snapshot against SPY and, when
/// available, the sector ETF benchmark. Uses intersected daily log returns.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrStkSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub symbol_sector: String,
    pub market_benchmark: String, // usually "SPY"
    pub sector_benchmark: String, // e.g. "XLK", empty when unavailable
    pub overlaps_spy_20d: usize,
    pub overlaps_spy_60d: usize,
    pub overlaps_spy_252d: usize,
    pub overlaps_sector_20d: usize,
    pub overlaps_sector_60d: usize,
    pub overlaps_sector_252d: usize,
    pub corr_spy_20d: f64,
    pub corr_spy_60d: f64,
    pub corr_spy_252d: f64,
    pub beta_spy_252d: f64,
    pub r_squared_spy_252d: f64,
    pub corr_sector_20d: f64,
    pub corr_sector_60d: f64,
    pub corr_sector_252d: f64,
    pub beta_sector_252d: f64,
    pub r_squared_sector_252d: f64,
    pub dominant_benchmark: String, // "SPY" | sector ETF | "NONE"
    pub correlation_label: String, // INDEX_LOCKSTEP | SECTOR_LOCKSTEP | MIXED | DIVERGENT | INSUFFICIENT_DATA
    pub note: String,
}

/// TLRANK — 30-day trading-liquidity rank vs sector peers.
/// Percentile rank of trailing 30-session average dollar volume within the
/// same sector. Higher ADV$ = deeper near-term liquidity = higher rank.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ThirtyDayLiquidityRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub window_days: i32,
    pub bars_used: usize,
    pub avg_30d_dollar_volume: f64,
    pub tier_label: String, // DEEP / LIQUID / MODERATE / THIN / ILLIQUID
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_dollar_volume: f64,
    pub sector_p25_dollar_volume: f64,
    pub sector_p75_dollar_volume: f64,
    pub percentile_rank: f64, // 0..100 (higher = deeper recent liquidity)
    pub rank_position: usize, // 1-based (1 = deepest recent liquidity)
    pub rank_label: String,   // standard decile ladder
    pub note: String,
}

/// CORRRANK — sector rank of benchmark linkage.
/// Percentile rank of 252d absolute correlation to one benchmark basis
/// (SPY or the mapped sector ETF) across same-sector peers.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub benchmark_name: String, // "SPY" or sector ETF
    pub benchmark_kind: String, // "MARKET" | "SECTOR_ETF"
    pub subject_corr_252d: f64,
    pub subject_abs_corr_252d: f64,
    pub subject_beta_252d: f64,
    pub subject_r_squared_252d: f64,
    pub subject_correlation_label: String, // copied from CORRSTK
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_abs_corr_252d: f64,
    pub sector_p25_abs_corr_252d: f64,
    pub sector_p75_abs_corr_252d: f64,
    pub percentile_rank: f64, // 0..100 (higher = tighter benchmark linkage)
    pub rank_position: usize, // 1-based (1 = most benchmark-linked)
    pub rank_label: String,   // standard decile ladder
    pub note: String,
}

// ── remaining cache-backed Godel parity surfaces ──

/// OPERANK_DELTA — operating margin trend rank vs sector peers.
/// Percentile rank of `MarginsSnapshot.operating_margin_change_pct`
/// within the same sector. Higher expansion in operating margin earns a
/// higher rank.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct OperatingMarginDeltaRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub basis: String, // copied from MARGINS: "annual" | "quarterly"
    pub latest_period: String,
    pub operating_margin_pct: f64,
    pub operating_margin_change_pct: f64,
    pub operating_trend_label: String, // copied from MARGINS
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_change_pct: f64,
    pub sector_p25_change_pct: f64,
    pub sector_p75_change_pct: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String, // TOP_DECILE .. BOTTOM_DECILE | INSUFFICIENT_DATA | NO_DATA
    pub note: String,
}

/// DIVACC — dividend growth acceleration.
/// Tracks the latest annual dividend-growth delta vs the prior year's
/// growth rate using cached dividend-payment history.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DividendAccelerationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub total_payments: usize,
    pub years_covered: usize,
    pub latest_year: i32,
    pub latest_annual_dividend: f64,
    pub latest_yoy_growth_pct: f64,
    pub prior_yoy_growth_pct: f64,
    pub acceleration_pct_pts: f64, // latest_yoy - prior_yoy
    pub recent_3y_avg_growth_pct: f64,
    pub prior_3y_avg_growth_pct: f64,
    pub acceleration_3y_avg_pct_pts: f64,
    pub consecutive_growth_years: usize,
    pub consistency_score_pct: f64,
    pub annual_rows: Vec<DivgAnnualRow>,
    pub divacc_label: String, // ACCELERATING | REACCELERATING | STABLE | DECELERATING | CUTTING | NO_HISTORY
    pub note: String,
}

/// EPSACC — EPS acceleration from cached quarterly financials.
/// Compares the latest quarterly EPS y/y growth rate against the prior
/// quarter's y/y growth rate to identify acceleration or deceleration in
/// the earnings trajectory.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct EpsAccelerationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub quarters_used: usize,
    pub latest_period: String,
    pub latest_eps: f64,
    pub prior_year_eps: f64,
    pub latest_yoy_growth_pct: f64,
    pub prior_yoy_growth_pct: f64,
    pub acceleration_pct_pts: f64,
    pub recent_2q_avg_yoy_growth_pct: f64,
    pub prior_2q_avg_yoy_growth_pct: f64,
    pub positive_yoy_quarters: usize,
    pub epsacc_label: String, // ACCELERATING | TURNAROUND | STABLE | DECELERATING | EARNINGS_PRESSURE | INSUFFICIENT_DATA
    pub note: String,
}

/// VRP — volatility risk premium snapshot using cached IVOL + RVCONE.
/// Pairs the current ATM implied volatility against realized-vol cone
/// levels to flag cheap/rich implied-vol regimes.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolRiskPremiumSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub current_atm_iv_pct: f64,
    pub iv_rank: f64,
    pub iv_percentile: f64,
    pub iv_observation_count: usize,
    pub rv20_pct: f64,
    pub rv60_pct: f64,
    pub rv252_pct: f64,
    pub rv20_percentile: f64,
    pub rv_cone_label: String,
    pub iv_minus_rv20_pct: f64,
    pub iv_to_rv20_ratio: f64,
    pub iv_minus_rv252_pct: f64,
    pub iv_to_rv252_ratio: f64,
    pub premium_label: String, // CHEAP_IV | FAIR_IV | RICH_IV | EXTREME_RICH | INSUFFICIENT_DATA
    pub note: String,
}

// ── short-interest history + trend rank ─────────────────

/// SHORTRANK_DELTA — short-interest trend rank vs sector peers.
/// Uses the change in `short_percent_of_float` over the trailing 180-day
/// window, risk-inverted so short covering (more negative delta) earns a
/// higher / safer rank.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShortInterestDeltaRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub lookback_days: i32,
    pub history_points_used: usize,
    pub history_start_date: String,
    pub history_end_date: String,
    pub latest_short_pct_of_float: f64,
    pub prior_short_pct_of_float: f64,
    pub delta_short_pct_points: f64,
    pub latest_short_ratio: f64,
    pub prior_short_ratio: f64,
    pub subject_trend_label: String, // HEAVY_COVERING | COVERING | STABLE | BUILDING | HEAVY_BUILD
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_delta_pct_pts: f64,
    pub sector_p25_delta_pct_pts: f64,
    pub sector_p75_delta_pct_pts: f64,
    pub percentile_rank: f64, // risk-inverted: lower delta -> higher / safer percentile
    pub rank_position: usize, // 1 = safest short-interest trend in sector
    pub rank_label: String,   // SAFEST_DECILE … RISKIEST_DECILE | INSUFFICIENT_DATA | NO_DATA
    pub note: String,
}

// ── insider ownership concentration parity ─────────────

/// INSIDERCONC — insider ownership concentration vs sector peers.
/// Estimates insider-held % from the latest known `shares_owned_after` per
/// reporter in cached INS rows, normalized by Fundamentals.shares_outstanding.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct InsiderConcentrationSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub latest_holdings_date: String,
    pub trade_rows_used: usize,
    pub reporters_covered: usize,
    pub reporters_holding_shares: usize,
    pub shares_outstanding: f64,
    pub total_estimated_insider_shares: f64,
    pub estimated_insider_pct_held: f64,
    pub largest_reporter: String,
    pub largest_reporter_shares: f64,
    pub largest_reporter_pct_of_outstanding: f64,
    pub largest_reporter_weight_pct: f64,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_pct_held: f64,
    pub sector_p25_pct_held: f64,
    pub sector_p75_pct_held: f64,
    pub percentile_rank: f64,
    pub rank_position: usize,
    pub rank_label: String, // TOP_DECILE .. BOTTOM_DECILE | INSUFFICIENT_DATA | NO_DATA
    pub note: String,
}

// ── beta/peg rank + HP 52wk/rvcone/calendar ──

/// BETARANK — Sector percentile rank of Fundamentals.beta, risk-inverted.
/// Lower beta earns a higher (safer) rank, mirroring SHRANK / LEVRANK /
/// RRK. Requires ≥3 sector peers with a non-None beta value.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BetaRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub subject_beta: Option<f64>,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_beta: f64,
    pub sector_p25_beta: f64,
    pub sector_p75_beta: f64,
    pub percentile_rank: f64, // risk-inverted: low beta → high pct
    pub rank_position: usize, // 1 = safest beta in sector
    pub rank_label: String,   // SAFEST_DECILE … RISKIEST_DECILE | INSUFFICIENT_DATA | NO_DATA
    pub note: String,
}

/// PEGRANK — Sector percentile rank of Fundamentals.peg_ratio.
/// Lower PEG (cheaper growth) earns a higher (better-value) rank. Not
/// covered by VAL (which uses P/E, Forward P/E, P/B, P/S, EV/EBITDA, FCFY).
/// Requires ≥3 sector peers with a positive finite peg_ratio.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PegRankSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub sector: String,
    pub subject_peg: Option<f64>,
    pub peers_considered: usize,
    pub peers_with_data: usize,
    pub sector_median_peg: f64,
    pub sector_p25_peg: f64,
    pub sector_p75_peg: f64,
    pub percentile_rank: f64, // value-inverted: low PEG → high pct
    pub rank_position: usize, // 1 = best value in sector
    pub rank_label: String,   // TOP_DECILE … BOTTOM_DECILE | INSUFFICIENT_DATA | NO_DATA
    pub note: String,
}

/// FHIGHLOW — 52-week high/low distance + proximity band.
/// Pure symbol-local HP stat over the trailing 253-session window.
/// Tracks max/min close + dates + current-vs-high/low distance + a
/// proximity label (AT_HIGH / NEAR_HIGH / MID_RANGE / NEAR_LOW / AT_LOW).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FiftyTwoWeekHighLowSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub latest_close: f64,
    pub high_52w: f64,
    pub high_52w_date: String,
    pub days_since_high: usize,
    pub low_52w: f64,
    pub low_52w_date: String,
    pub days_since_low: usize,
    pub pct_from_high: f64, // (latest - high) / high × 100 — negative or 0
    pub pct_from_low: f64,  // (latest - low) / low × 100 — positive or 0
    pub range_position_pct: f64, // (latest - low) / (high - low) × 100
    pub proximity_label: String, // "AT_HIGH" | "NEAR_HIGH" | "MID_RANGE" | "NEAR_LOW" | "AT_LOW" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// RVCONE — Multi-horizon Realized Volatility Cone.
/// Pure symbol-local HP stat. Computes 20d/60d/120d/252d annualized
/// realized volatility (stdev of log returns × √252) from cached bars,
/// plus a cone-position percentile of the latest 20d RV vs the rolling
/// distribution of 20d RVs over the full window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealizedVolConeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub latest_close: f64,
    pub rv20_pct: f64, // annualized realized vol over 20 sessions
    pub rv60_pct: f64,
    pub rv120_pct: f64,
    pub rv252_pct: f64,
    pub rv20_min_pct: f64,    // min of all rolling 20d RVs in the window
    pub rv20_median_pct: f64, // median of rolling 20d RVs
    pub rv20_max_pct: f64,    // max of rolling 20d RVs
    pub rv20_percentile: f64, // latest 20d RV percentile vs rolling distribution (0-100)
    pub cone_label: String, // "COMPRESSED" | "BELOW_AVG" | "TYPICAL" | "ELEVATED" | "EXTREME" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// CALPB — Calendar Period Breakdowns.
/// Pure symbol-local HP stat that aligns to calendar boundaries rather
/// than rolling-session offsets. Emits MTD/QTD/current-year returns
/// plus prior-quarter and prior-year returns for comparison, and a
/// momentum-vs-prior-period label.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CalendarPeriodBreakdownSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub latest_close: f64,
    pub mtd_pct: f64,           // current month-to-date return
    pub qtd_pct: f64,           // current quarter-to-date return
    pub ytd_pct: f64,           // current year-to-date return (calendar)
    pub prior_quarter_pct: f64, // prior calendar quarter return
    pub prior_year_pct: f64,    // prior calendar year return
    pub current_year: String,
    pub current_quarter: String, // e.g. "Q2"
    pub momentum_label: String, // "ACCELERATING" | "STEADY" | "DECELERATING" | "REVERSING" | "INSUFFICIENT_DATA"
    pub note: String,
}

// ── HP return-distribution + behavior stats ──

/// RETSKEW — Return distribution skewness (third standardized moment).
/// Pure symbol-local HP stat over the trailing 253-session window of log
/// returns. Positive skew → large upside outliers; negative skew → large
/// downside outliers. Complements RVCONE (second moment) and RETKURT
/// (fourth moment) with a third-moment tail-asymmetry view.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReturnSkewnessSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize, // number of log returns used
    pub mean_log_return: f64,
    pub stdev_log_return: f64,
    pub skewness: f64,            // third standardized moment
    pub positive_return_pct: f64, // share of up-days
    pub largest_up_pct: f64,      // max log-return (×100)
    pub largest_down_pct: f64,    // min log-return (×100)
    pub skew_label: String, // "STRONG_LEFT" | "LEFT" | "SYMMETRIC" | "RIGHT" | "STRONG_RIGHT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// RETKURT — Return distribution excess kurtosis (fourth standardized moment - 3).
/// Pure symbol-local HP stat over the trailing 253-session window of log
/// returns. High excess kurtosis → fat-tailed distribution with more
/// extreme moves than a normal would predict.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ReturnKurtosisSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub mean_log_return: f64,
    pub stdev_log_return: f64,
    pub excess_kurtosis: f64,        // fourth standardized moment - 3
    pub outlier_2sigma_count: usize, // count of |z| > 2 returns
    pub outlier_3sigma_count: usize, // count of |z| > 3 returns
    pub outlier_2sigma_pct: f64,     // share of |z| > 2 returns (normal ≈ 4.55%)
    pub kurt_label: String, // "PLATYKURTIC" | "NORMAL" | "MILD_FAT" | "FAT" | "EXTREME_FAT" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// TAILR — Tail ratio = 95th pct return / |5th pct return|.
/// Pure symbol-local HP stat. Ratio > 1 → upside tail dominates;
/// < 1 → downside tail dominates. Complements RETSKEW with a
/// non-parametric quantile-based view of tail asymmetry.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TailRatioSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub pct_95_return: f64,    // 95th percentile return (as %)
    pub pct_05_return: f64,    // 5th percentile return (as %)
    pub pct_99_return: f64,    // 99th percentile return
    pub pct_01_return: f64,    // 1st percentile return
    pub tail_ratio: f64,       // pct_95 / |pct_05|
    pub tail_ratio_99_01: f64, // pct_99 / |pct_01|
    pub bias_label: String, // "DOWNSIDE_HEAVY" | "SLIGHT_DOWNSIDE" | "BALANCED" | "SLIGHT_UPSIDE" | "UPSIDE_HEAVY" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// RUNLEN — Up/down day run length statistics.
/// Pure symbol-local HP stat. Average and longest runs of consecutive
/// up-days and down-days over the trailing 253-session window. Long
/// runs → trending regime; short runs → choppy / mean-reverting.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RunLengthSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub avg_up_run: f64,
    pub avg_down_run: f64,
    pub longest_up_run: usize,
    pub longest_down_run: usize,
    pub up_runs_count: usize,
    pub down_runs_count: usize,
    pub current_run_length: i32, // positive = up run, negative = down run, 0 = flat
    pub trend_label: String, // "CHOPPY" | "MIXED" | "TRENDING" | "STRONG_TRENDING" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// DAYRANGE — Daily range analysis.
/// Pure symbol-local HP stat. Average (high - low) / close over 60
/// sessions vs 252-session baseline. Ratio < 1 → compressed (expect
/// breakout); > 1 → expanded (volatility regime).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DailyRangeSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub bars_used: usize,
    pub avg_range_60_pct: f64,    // avg (high-low)/close × 100 over 60d
    pub avg_range_252_pct: f64,   // avg (high-low)/close × 100 over 252d
    pub latest_range_pct: f64,    // latest bar's (high-low)/close × 100
    pub compression_ratio: f64,   // 60d avg / 252d avg (1.0 = neutral)
    pub widest_range_pct: f64,    // max (high-low)/close × 100 in window
    pub narrowest_range_pct: f64, // min (high-low)/close × 100 in window
    pub range_label: String, // "TIGHT" | "COMPRESSED" | "NORMAL" | "EXPANDED" | "VERY_EXPANDED" | "INSUFFICIENT_DATA"
    pub note: String,
}
