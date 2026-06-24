//! Chart-local leaf data types (ADR-125 Target 2, slice 1).
//!
//! `Bar` / `ChartType` / `Timeframe` are the foundational data types the chart
//! rendering and state layers are built on. They are pure data + simple inherent
//! methods (no egui, no engine, no `TyphooNApp`), so they move first; `typhoon-native`
//! re-exports them from `app::types` so existing call sites are unchanged.

/// A single OHLCV bar.
#[derive(Clone, Debug)]
pub struct Bar {
    pub ts_ms: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: f64,
}

/// Chart rendering style.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ChartType {
    Candle,
    HeikinAshi,
    Line,
    OhlcBars,
    Renko,
}

impl ChartType {
    pub fn label(self) -> &'static str {
        match self {
            ChartType::Candle => "Candle",
            ChartType::HeikinAshi => "Heikin-Ashi",
            ChartType::Line => "Line",
            ChartType::OhlcBars => "OHLC Bars",
            ChartType::Renko => "Renko",
        }
    }
}

/// Available timeframes for the selector toolbar.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Hash)]
pub enum Timeframe {
    // Native timeframes (stored directly in cache / returned by brokers)
    M1,
    M5,
    M15,
    M30,
    H1,
    H4,
    D1,
    W1,
    MN1,
}

impl Timeframe {
    pub fn label(self) -> &'static str {
        match self {
            Timeframe::M1 => "M1",
            Timeframe::M5 => "M5",
            Timeframe::M15 => "M15",
            Timeframe::M30 => "M30",
            Timeframe::H1 => "H1",
            Timeframe::H4 => "H4",
            Timeframe::D1 => "D1",
            Timeframe::W1 => "W1",
            Timeframe::MN1 => "MN1",
        }
    }

    /// All timeframes for dropdown display.
    pub fn all() -> &'static [Timeframe] {
        &[
            Timeframe::M1,
            Timeframe::M5,
            Timeframe::M15,
            Timeframe::M30,
            Timeframe::H1,
            Timeframe::H4,
            Timeframe::D1,
            Timeframe::W1,
            Timeframe::MN1,
        ]
    }

    /// Parse from label string (for session restore, command palette).
    pub fn from_label(s: &str) -> Option<Self> {
        Self::all()
            .iter()
            .find(|tf| tf.label().eq_ignore_ascii_case(s))
            .copied()
    }

    /// Timeframe in minutes.
    pub fn minutes(self) -> u32 {
        match self {
            Timeframe::M1 => 1,
            Timeframe::M5 => 5,
            Timeframe::M15 => 15,
            Timeframe::M30 => 30,
            Timeframe::H1 => 60,
            Timeframe::H4 => 240,
            Timeframe::D1 => 1440,
            Timeframe::W1 => 10080,
            Timeframe::MN1 => 43200,
        }
    }

    /// Cache key suffix for the native timeframe.
    pub fn cache_suffix(self) -> &'static str {
        match self {
            Timeframe::M1 => "1Min",
            Timeframe::M5 => "5Min",
            Timeframe::M15 => "15Min",
            Timeframe::M30 => "30Min",
            Timeframe::H1 => "1Hour",
            Timeframe::H4 => "4Hour",
            Timeframe::D1 => "1Day",
            Timeframe::W1 => "1Week",
            Timeframe::MN1 => "1Month",
        }
    }

    /// Coarse timeframe group rank (used by Previous Candle Levels).
    pub fn group_rank(self) -> u8 {
        match self {
            Timeframe::M1 | Timeframe::M5 | Timeframe::M15 | Timeframe::M30 => 0,
            Timeframe::H1 | Timeframe::H4 => 1,
            Timeframe::D1 => 2,
            Timeframe::W1 => 3,
            Timeframe::MN1 => 4,
        }
    }
}

/// Bare ticker from a `source:symbol:timeframe` (or `symbol:timeframe`) cache key
/// (ADR-125 Target 2, slice 7b). A symbol-key primitive shared by the chart renderers
/// (axis ticker label, time-axis) and native cache/market-data code; native re-exports it.
pub fn bare_symbol_from_key(key: &str) -> String {
    let parts: Vec<&str> = key.split(':').collect();
    match parts.as_slice() {
        [_src, sym, _tf] => (*sym).to_string(),
        [sym, _tf] => (*sym).to_string(),
        _ => key.to_string(),
    }
}
