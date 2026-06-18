use serde::{Deserialize, Serialize};

// ── Godel Parity Round 9 ─────────────────────────────────────────
// SEAG / COR / TRA / TECH / SKEW surfaces — all pure compute over existing
// HP / DVD / OMON caches, zero new API dependencies.

/// SEAG — one month's historical seasonality bucket (Jan..Dec).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeasonalityMonth {
    pub month: u32,          // 1..12
    pub label: String,       // "Jan", "Feb", …
    pub avg_return_pct: f64, // mean monthly return across years
    pub median_return_pct: f64,
    pub stdev_pct: f64,
    pub positive_years: usize,
    pub total_years: usize,
    pub best_return_pct: f64,
    pub worst_return_pct: f64,
}

/// SEAG — one day-of-week historical bucket (Mon..Fri).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeasonalityDow {
    pub dow: u32,            // 1..7 (Mon=1, Sun=7)
    pub label: String,       // "Mon", "Tue", …
    pub avg_return_pct: f64, // mean daily log-return
    pub positive_days: usize,
    pub total_days: usize,
}

/// SEAG — Seasonality analysis snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SeasonalitySnapshot {
    pub symbol: String,
    pub as_of: String,
    pub years_covered: usize,
    pub months: Vec<SeasonalityMonth>,
    pub dow: Vec<SeasonalityDow>,
    pub best_month: String,
    pub worst_month: String,
    pub note: String,
}

/// COR — one pairwise correlation cell.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationCell {
    pub peer_symbol: String,
    pub correlation: f64, // Pearson on daily log-returns
    pub n_observations: usize,
    pub beta_vs_peer: f64, // slope of ln(subject) vs ln(peer)
}

/// COR — Correlation matrix for a subject vs its peer set.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CorrelationMatrix {
    pub symbol: String,
    pub as_of: String,
    pub window_days: usize, // e.g. 252 (1Y)
    pub cells: Vec<CorrelationCell>,
    pub mean_correlation: f64, // average |ρ| across cells
    pub highest_corr_symbol: String,
    pub lowest_corr_symbol: String,
    pub note: String,
}

/// TRA — one total-return window (price return + dividend yield).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TotalReturnWindow {
    pub label: String, // "1M" / "3M" / "6M" / "YTD" / "1Y" / "3Y" / "5Y"
    pub trading_days: usize,
    pub price_return_pct: f64,
    pub dividend_yield_pct: f64, // dividends paid in window / start price × 100
    pub total_return_pct: f64,   // price + dividend yield (simple, not compound)
    pub annualized_pct: f64,     // annualized for windows ≥ 1Y, else simple
    pub dividends_paid: f64,     // cash per share in window
    pub n_dividends: usize,
}

/// TRA — Total return analysis snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TotalReturnSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub last_close: f64,
    pub trailing_12m_dividends: f64,
    pub trailing_12m_yield_pct: f64,
    pub windows: Vec<TotalReturnWindow>,
    pub note: String,
}

/// TECH — one indicator value with its signal interpretation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TechnicalIndicator {
    pub name: String, // "RSI(14)", "MACD(12,26,9)", "BB(20,2)", "ATR(14)", "ADX(14)", "Stoch(14,3)"
    pub value: f64,   // primary value (for MACD this is the histogram)
    pub value_secondary: f64, // signal line / middle band / +DI / etc.
    pub value_tertiary: f64, // -DI / lower band / …
    pub signal: String, // "overbought" / "oversold" / "bullish" / "bearish" / "neutral"
    pub note: String, // short contextual hint
}

/// TECH — Technical indicator snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TechnicalSnapshot {
    pub symbol: String,
    pub as_of: String,
    pub last_close: f64,
    pub indicators: Vec<TechnicalIndicator>,
    pub trend_summary: String, // short synthesized label
    pub note: String,
}

/// SKEW — one strike row on a volatility smile curve.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkewPoint {
    pub strike: f64,
    pub moneyness_pct: f64, // (strike / underlying - 1) × 100
    pub call_iv_pct: f64,
    pub put_iv_pct: f64,
    pub combined_iv_pct: f64, // average of call/put when both present
}

/// SKEW — one expiry's full smile + summary stats.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SkewExpiry {
    pub expiration: String,
    pub days_to_expiry: i64,
    pub atm_iv_pct: f64,
    pub points: Vec<SkewPoint>,
    pub put_call_skew_25d_pct: f64, // 25-delta put IV − 25-delta call IV (placeholder using ±10% OTM)
    pub term_note: String,
}

/// SKEW — Implied-volatility skew snapshot for a symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VolatilitySkew {
    pub symbol: String,
    pub as_of: String,
    pub underlying_price: f64,
    pub expiries: Vec<SkewExpiry>,
    pub note: String,
}
