use serde::{Deserialize, Serialize};

// ── Godel Parity Round 10 ───────────────────────────────────────────

/// LEV — one leverage / coverage ratio row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeverageRatio {
    pub name: String,
    pub value: f64,
    pub peer_median: f64, // 0.0 when unknown
    pub signal: String,   // "HEALTHY" | "ELEVATED" | "STRETCHED" | "NEUTRAL"
    pub note: String,
}

/// LEV — full leverage / solvency snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LeverageSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub total_debt: f64,
    pub net_debt: f64,
    pub ebitda_ttm: f64,
    pub interest_expense_ttm: f64,
    pub total_equity: f64,
    pub ratios: Vec<LeverageRatio>,
    pub solvency_summary: String,
    pub note: String,
}

/// ACRL — one quarter's earnings-quality row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccrualPeriod {
    pub period: String, // "FY2024" or "Q3 2024"
    pub date: String,   // YYYY-MM-DD
    pub net_income: f64,
    pub free_cash_flow: f64,
    pub fcf_to_ni_ratio: f64,     // FCF / NI
    pub cash_conversion_pct: f64, // FCF / NI × 100
    pub accruals: f64,            // NI - FCF
    pub quality_label: String,    // "HIGH" | "MEDIUM" | "LOW" | "NEGATIVE_NI"
}

/// ACRL — earnings quality snapshot (accruals vs cash flow conversion).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AccrualsSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub ttm_net_income: f64,
    pub ttm_free_cash_flow: f64,
    pub ttm_cash_conversion_pct: f64,
    pub avg_cash_conversion_pct: f64, // across the tracked periods
    pub periods: Vec<AccrualPeriod>,
    pub trend_label: String, // "IMPROVING" | "STABLE" | "DETERIORATING" | "MIXED"
    pub note: String,
}

/// RVOL — one realized-volatility window observation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealizedVolWindow {
    pub label: String, // "20d" / "60d" / "120d" / "252d"
    pub trading_days: usize,
    pub realized_vol_pct: f64, // annualized
    pub percentile: f64,       // 0..=100 — cone rank vs the full history of this window
    pub n_observations: usize,
}

/// RVOL — realized volatility + IV/RV gap snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RealizedVolSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub last_close: f64,
    pub current_atm_iv_pct: f64, // from cached IVOL, 0.0 when unknown
    pub iv_rv_gap_pct: f64,      // IV − RV(20d)
    pub iv_rv_ratio: f64,        // IV / RV(20d)
    pub windows: Vec<RealizedVolWindow>,
    pub regime_label: String, // "CHEAP_IV" | "FAIR_IV" | "RICH_IV" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// FCFY — one dividend coverage / FCF yield row.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FcfYieldPeriod {
    pub period: String,
    pub date: String,
    pub free_cash_flow: f64,
    pub dividends_paid: f64,
    pub payout_from_fcf_pct: f64, // dividends_paid / FCF × 100 (absolute cash-out ratio)
    pub payout_from_ni_pct: f64,  // dividends_paid / NI × 100
    pub fcf_yield_pct: f64, // FCF / market_cap_at_period × 100 (only TTM-level rows populate this)
}

/// FCFY — FCF yield + dividend sustainability snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FcfYieldSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub market_cap: f64,
    pub ttm_free_cash_flow: f64,
    pub ttm_dividends_paid: f64,
    pub ttm_fcf_yield_pct: f64,
    pub ttm_dividend_yield_pct: f64,
    pub ttm_payout_from_fcf_pct: f64,
    pub ttm_payout_from_ni_pct: f64,
    pub fcf_cagr_5y_pct: f64, // 0.0 when <5 years of annuals
    pub periods: Vec<FcfYieldPeriod>,
    pub sustainability_label: String, // "SAFE" | "STRETCHED" | "UNSUSTAINABLE" | "NO_DIVIDEND"
    pub note: String,
}

/// SHRT — short interest + days-to-cover + squeeze signal snapshot.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ShortInterestSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub shares_outstanding: f64,
    pub shares_float: f64,
    pub short_shares: f64,
    pub short_percent_of_float: f64,
    pub avg_daily_volume_20d: f64,
    pub days_to_cover: f64,         // short_shares / avg_daily_volume_20d
    pub short_ratio_reported: f64,  // from Fundamentals (vendor-provided, may differ)
    pub utilization_proxy_pct: f64, // short / float × 100 (same as short_percent_of_float but normalized)
    pub squeeze_risk_label: String, // "LOW" | "ELEVATED" | "HIGH" | "EXTREME" | "INSUFFICIENT_DATA"
    pub note: String,
}

/// One short-interest history observation for a symbol.
/// Stored as a compact per-symbol time series and fed by fundamentals scrapes
/// plus explicit short-interest fetches when available.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ShortInterestHistoryPoint {
    pub as_of: String, // YYYY-MM-DD
    pub short_percent_of_float: f64,
    pub short_ratio: f64,
    pub shares_outstanding: f64,
}
