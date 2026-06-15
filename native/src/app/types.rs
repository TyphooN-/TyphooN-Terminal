//! Extracted from app.rs: types helpers.

use super::*;

// ─── types ───────────────────────────────────────────────────────────────────

/// A single OHLCV bar.
#[derive(Clone, Debug)]
pub struct Bar {
    pub(crate) ts_ms: i64,
    pub(crate) open: f64,
    pub(crate) high: f64,
    pub(crate) low: f64,
    pub(crate) close: f64,
    pub(crate) volume: f64,
}

/// Chart rendering style.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum ChartType {
    Candle,
    HeikinAshi,
    Line,
    OhlcBars,
    Renko,
}

impl ChartType {
    pub(crate) fn label(self) -> &'static str {
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
pub(crate) enum Timeframe {
    // Base timeframes (stored natively in cache)
    M1,
    M5,
    M15,
    M30,
    H1,
    H4,
    D1,
    W1,
    MN1,
    // Custom aggregated timeframes (built from base TFs on load)
    M2,
    M3,
    M10,
    M20,
    M45,
    H2,
    H3,
    H6,
    H8,
    H12,
    D2,
    D3,
    D5,
    D10,
    W2,
    W3,
    MN2,
    MN3,
    MN6,
    Y1,
    Y2,
    Y3,
    Y5,
    Y10,
}

impl Timeframe {
    pub(crate) fn label(self) -> &'static str {
        match self {
            Timeframe::M1 => "M1",
            Timeframe::M2 => "M2",
            Timeframe::M3 => "M3",
            Timeframe::M5 => "M5",
            Timeframe::M10 => "M10",
            Timeframe::M15 => "M15",
            Timeframe::M20 => "M20",
            Timeframe::M30 => "M30",
            Timeframe::M45 => "M45",
            Timeframe::H1 => "H1",
            Timeframe::H2 => "H2",
            Timeframe::H3 => "H3",
            Timeframe::H4 => "H4",
            Timeframe::H6 => "H6",
            Timeframe::H8 => "H8",
            Timeframe::H12 => "H12",
            Timeframe::D1 => "D1",
            Timeframe::D2 => "D2",
            Timeframe::D3 => "D3",
            Timeframe::D5 => "D5",
            Timeframe::D10 => "D10",
            Timeframe::W1 => "W1",
            Timeframe::W2 => "W2",
            Timeframe::W3 => "W3",
            Timeframe::MN1 => "MN1",
            Timeframe::MN2 => "MN2",
            Timeframe::MN3 => "MN3",
            Timeframe::MN6 => "MN6",
            Timeframe::Y1 => "Y1",
            Timeframe::Y2 => "Y2",
            Timeframe::Y3 => "Y3",
            Timeframe::Y5 => "Y5",
            Timeframe::Y10 => "Y10",
        }
    }

    /// All timeframes for dropdown display, organized by group.
    pub(crate) fn all() -> &'static [Timeframe] {
        &[
            Timeframe::M1,
            Timeframe::M2,
            Timeframe::M3,
            Timeframe::M5,
            Timeframe::M10,
            Timeframe::M15,
            Timeframe::M20,
            Timeframe::M30,
            Timeframe::M45,
            Timeframe::H1,
            Timeframe::H2,
            Timeframe::H3,
            Timeframe::H4,
            Timeframe::H6,
            Timeframe::H8,
            Timeframe::H12,
            Timeframe::D1,
            Timeframe::D2,
            Timeframe::D3,
            Timeframe::D5,
            Timeframe::D10,
            Timeframe::W1,
            Timeframe::W2,
            Timeframe::W3,
            Timeframe::MN1,
            Timeframe::MN2,
            Timeframe::MN3,
            Timeframe::MN6,
            Timeframe::Y1,
            Timeframe::Y2,
            Timeframe::Y3,
            Timeframe::Y5,
            Timeframe::Y10,
        ]
    }

    /// Parse from label string (for session restore, command palette).
    pub(crate) fn from_label(s: &str) -> Option<Self> {
        Self::all()
            .iter()
            .find(|tf| tf.label().eq_ignore_ascii_case(s))
            .copied()
    }

    /// Timeframe in minutes.
    pub(crate) fn minutes(self) -> u32 {
        match self {
            Timeframe::M1 => 1,
            Timeframe::M2 => 2,
            Timeframe::M3 => 3,
            Timeframe::M5 => 5,
            Timeframe::M10 => 10,
            Timeframe::M15 => 15,
            Timeframe::M20 => 20,
            Timeframe::M30 => 30,
            Timeframe::M45 => 45,
            Timeframe::H1 => 60,
            Timeframe::H2 => 120,
            Timeframe::H3 => 180,
            Timeframe::H4 => 240,
            Timeframe::H6 => 360,
            Timeframe::H8 => 480,
            Timeframe::H12 => 720,
            Timeframe::D1 => 1440,
            Timeframe::D2 => 2880,
            Timeframe::D3 => 4320,
            Timeframe::D5 => 7200,
            Timeframe::D10 => 14400,
            Timeframe::W1 => 10080,
            Timeframe::W2 => 20160,
            Timeframe::W3 => 30240,
            Timeframe::MN1 => 43200,
            Timeframe::MN2 => 86400,
            Timeframe::MN3 => 129600,
            Timeframe::MN6 => 259200,
            Timeframe::Y1 => 525600,
            Timeframe::Y2 => 1051200,
            Timeframe::Y3 => 1576800,
            Timeframe::Y5 => 2628000,
            Timeframe::Y10 => 5256000,
        }
    }

    /// Cache key suffix. For custom TFs, returns the BASE TF suffix.
    pub(crate) fn cache_suffix(self) -> &'static str {
        match self {
            Timeframe::M1 | Timeframe::M2 | Timeframe::M3 => "1Min",
            Timeframe::M5 | Timeframe::M10 | Timeframe::M20 => "5Min",
            Timeframe::M15 | Timeframe::M45 => "15Min",
            Timeframe::M30 => "30Min",
            Timeframe::H1 | Timeframe::H2 | Timeframe::H3 | Timeframe::H6 => "1Hour",
            Timeframe::H4 | Timeframe::H8 | Timeframe::H12 => "4Hour",
            Timeframe::D1 | Timeframe::D2 | Timeframe::D3 | Timeframe::D5 | Timeframe::D10 => {
                "1Day"
            }
            Timeframe::W1 | Timeframe::W2 | Timeframe::W3 => "1Week",
            Timeframe::MN1
            | Timeframe::MN2
            | Timeframe::MN3
            | Timeframe::MN6
            | Timeframe::Y1
            | Timeframe::Y2
            | Timeframe::Y3
            | Timeframe::Y5
            | Timeframe::Y10 => "1Month",
        }
    }

    /// Aggregation: (base_tf, factor). None for base TFs.
    pub(crate) fn aggregation(self) -> Option<usize> {
        match self {
            Timeframe::M2 => Some(2),
            Timeframe::M3 => Some(3),
            Timeframe::M10 => Some(2),
            Timeframe::M20 => Some(4),
            Timeframe::M45 => Some(3),
            Timeframe::H2 => Some(2),
            Timeframe::H3 => Some(3),
            Timeframe::H6 => Some(6),
            Timeframe::H8 => Some(2),
            Timeframe::H12 => Some(3),
            Timeframe::D2 => Some(2),
            Timeframe::D3 => Some(3),
            Timeframe::D5 => Some(5),
            Timeframe::D10 => Some(10),
            Timeframe::W2 => Some(2),
            Timeframe::W3 => Some(3),
            Timeframe::MN2 => Some(2),
            Timeframe::MN3 => Some(3),
            Timeframe::MN6 => Some(6),
            Timeframe::Y1 => Some(12),
            Timeframe::Y2 => Some(24),
            Timeframe::Y3 => Some(36),
            Timeframe::Y5 => Some(60),
            Timeframe::Y10 => Some(120),
            _ => None,
        }
    }

    /// Coarse timeframe group used to decide which "previous candle level" lines
    /// to draw. Mirrors PreviousCandleLevels.mqh: a level is shown only when the
    /// chart's period sits in a *lower* group than the level's timeframe, so a
    /// sub-hour chart shows H1/H4/D/W/MN, an hourly chart shows D/W/MN, a daily
    /// chart shows W/MN, a weekly chart shows MN, and monthly+/yearly charts show
    /// none. H12 is grouped with the daily timeframes, matching the reference.
    /// Ranks: 0 sub-hour, 1 hour, 2 day, 3 week, 4 month, 5 year.
    pub(crate) fn group_rank(self) -> u8 {
        match self {
            Timeframe::M1
            | Timeframe::M2
            | Timeframe::M3
            | Timeframe::M5
            | Timeframe::M10
            | Timeframe::M15
            | Timeframe::M20
            | Timeframe::M30
            | Timeframe::M45 => 0,
            Timeframe::H1
            | Timeframe::H2
            | Timeframe::H3
            | Timeframe::H4
            | Timeframe::H6
            | Timeframe::H8 => 1,
            Timeframe::H12
            | Timeframe::D1
            | Timeframe::D2
            | Timeframe::D3
            | Timeframe::D5
            | Timeframe::D10 => 2,
            Timeframe::W1 | Timeframe::W2 | Timeframe::W3 => 3,
            Timeframe::MN1 | Timeframe::MN2 | Timeframe::MN3 | Timeframe::MN6 => 4,
            Timeframe::Y1 | Timeframe::Y2 | Timeframe::Y3 | Timeframe::Y5 | Timeframe::Y10 => 5,
        }
    }
}

pub(crate) fn alpaca_incremental_fetch_limit(
    timeframe: &str,
    after_timestamp: Option<&str>,
) -> u32 {
    alpaca_incremental_fetch_limit_at(chrono::Utc::now().timestamp(), timeframe, after_timestamp)
}

/// Log severity level.
#[derive(Clone, Copy, Debug)]
pub(crate) enum LogLevel {
    Info,
    Warn,
    Error,
    Trade, // ADR-094: fills, executions
    Alert, // ADR-094: triggered alerts
}

/// Log filter for the bottom panel dropdown.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum LogFilter {
    All,
    Info,
    Warn,
    Error,
    Trade,
    Alert,
}

/// A single log entry displayed in the bottom panel.
#[derive(Clone, Debug)]
pub(crate) struct LogEntry {
    pub(crate) level: LogLevel,
    pub(crate) msg: String,
    /// PERF: pre-formatted display text `"[HH:MM:SS] <icon> <msg>"` built once
    /// at construction. The bottom-panel log was calling `format!()` per entry
    /// per frame (~200 entries × 60fps = 12k allocs/sec) for a string that
    /// never changes once created. `timestamp` is folded into this buffer.
    pub(crate) display: String,
}

/// Indicator-based alert condition.
#[derive(Clone)]
pub(crate) struct IndicatorAlert {
    pub(crate) symbol: String,
    pub(crate) timeframe: String,
    pub(crate) indicator: String, // "RSI", "MACD", "Price", "Fisher", etc.
    pub(crate) condition: String, // "crosses_above", "crosses_below", "greater_than", "less_than"
    pub(crate) threshold: f64,
    pub(crate) active: bool,
    pub(crate) triggered: bool,
    pub(crate) last_value: Option<f64>,
}

pub(crate) const ALERT_INDICATORS: &[&str] = &[
    "Price",
    "RSI",
    "Fisher",
    "MACD",
    "ATR",
    "ADX",
    "Stochastic %K",
    "CCI",
    "Volume",
];
pub(crate) const ALERT_CONDITIONS: &[&str] = &[
    "crosses above",
    "crosses below",
    "greater than",
    "less than",
];

/// Trade Journal entry for tracking live/paper trades with notes.
#[derive(Clone)]
pub(crate) struct JournalEntry {
    pub(crate) timestamp: String,
    pub(crate) symbol: String,
    pub(crate) side: String, // "BUY" or "SELL"
    pub(crate) qty: f64,
    pub(crate) entry_price: f64,
    pub(crate) exit_price: Option<f64>,
    pub(crate) pnl: Option<f64>,
    pub(crate) strategy: String,
    pub(crate) notes: String,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct KrakenCostBasis {
    pub qty: f64,
    pub cost: f64,
}

impl KrakenCostBasis {
    pub fn avg_price(self) -> Option<f64> {
        (self.qty > 0.0 && self.cost > 0.0).then_some(self.cost / self.qty)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct UnresolvablePair {
    pub broker: String,
    pub symbol: String,
    pub timeframe: String,
    pub reason: String,
    pub ts: i64,
}

pub(crate) fn unresolvable_pair_key(broker: &str, symbol: &str, timeframe: &str) -> String {
    format!(
        "{}:{}:{}",
        broker.to_ascii_lowercase(),
        normalize_market_data_symbol(symbol)
            .replace('/', "")
            .to_ascii_uppercase(),
        normalize_sync_timeframe_key(timeframe).unwrap_or(timeframe)
    )
}

impl LogEntry {
    pub(crate) fn now_ts() -> String {
        chrono::Local::now().format("%H:%M:%S").to_string()
    }
    pub(crate) fn new(level: LogLevel, msg: String) -> Self {
        let timestamp = Self::now_ts();
        let icon: &'static str = match level {
            LogLevel::Info => "\u{2139}",
            LogLevel::Warn => "\u{26A0}",
            LogLevel::Error => "\u{2716}",
            LogLevel::Trade => "\u{1F4B0}",
            LogLevel::Alert => "\u{1F514}",
        };
        let mut display = String::with_capacity(timestamp.len() + msg.len() + 12);
        display.push('[');
        display.push_str(&timestamp);
        display.push_str("] ");
        display.push_str(icon);
        display.push(' ');
        display.push_str(&msg);
        match level {
            LogLevel::Info | LogLevel::Trade | LogLevel::Alert => tracing::info!("{}", msg),
            LogLevel::Warn => tracing::warn!("{}", msg),
            LogLevel::Error => tracing::error!("{}", msg),
        }
        Self {
            level,
            msg,
            display,
        }
    }
    pub(crate) fn info(msg: impl Into<String>) -> Self {
        Self::new(LogLevel::Info, msg.into())
    }
    pub(crate) fn warn(msg: impl Into<String>) -> Self {
        Self::new(LogLevel::Warn, msg.into())
    }
    pub(crate) fn err(msg: impl Into<String>) -> Self {
        Self::new(LogLevel::Error, msg.into())
    }
    pub(crate) fn trade(msg: impl Into<String>) -> Self {
        Self::new(LogLevel::Trade, msg.into())
    }
    pub(crate) fn alert(msg: impl Into<String>) -> Self {
        Self::new(LogLevel::Alert, msg.into())
    }

    pub(crate) fn color(&self) -> egui::Color32 {
        match self.level {
            LogLevel::Info => egui::Color32::from_rgb(160, 200, 160),
            LogLevel::Warn => egui::Color32::from_rgb(255, 200, 50),
            LogLevel::Error => egui::Color32::from_rgb(255, 80, 80),
            LogLevel::Trade => egui::Color32::from_rgb(80, 220, 120),
            LogLevel::Alert => egui::Color32::from_rgb(255, 165, 0),
        }
    }

    pub(crate) fn matches_filter(&self, filter: LogFilter) -> bool {
        match filter {
            LogFilter::All => true,
            LogFilter::Info => matches!(self.level, LogLevel::Info),
            LogFilter::Warn => matches!(self.level, LogLevel::Warn),
            LogFilter::Error => matches!(self.level, LogLevel::Error),
            LogFilter::Trade => matches!(self.level, LogLevel::Trade),
            LogFilter::Alert => matches!(self.level, LogLevel::Alert),
        }
    }
}

// ── ADR-094: Result Cards ──────────────────────────────────────────

/// Structured analytics result displayed above the log panel.
#[derive(Clone)]
pub(crate) enum ResultCard {
    /// Key-value metrics (VAR, RISK_CALC, MARGIN, COMPOUND)
    Summary {
        title: String,
        metrics: Vec<(String, String, egui::Color32)>, // (label, value, color)
    },
    /// Sortable table (SCREENER, OUTLIERS, STRESS_TEST)
    Table {
        title: String,
        headers: Vec<String>,
        rows: Vec<Vec<String>>,
        sort_col: usize,
        sort_asc: bool,
    },
    /// Mini sparkline chart (FRED, BACKTEST equity)
    Chart {
        title: String,
        label: String,
        values: Vec<f64>,
    },
}

// ── ADR-094: Toast Notifications ───────────────────────────────────

/// Overlay toast notification.
#[derive(Clone)]
pub(crate) struct Toast {
    pub(crate) message: String,
    pub(crate) color: egui::Color32,
    pub(crate) created: std::time::Instant,
    pub(crate) duration: std::time::Duration,
    pub(crate) dismissable: bool,
    pub(crate) dismissed: bool,
}

impl Toast {
    pub(crate) fn is_expired(&self) -> bool {
        self.dismissed || self.created.elapsed() > self.duration
    }
}

// ── ADR-094: Command Palette Context ───────────────────────────────

/// Context for right-click command palette filtering.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum PaletteContext {
    /// Full command list (backtick key)
    Global,
    /// Right-clicked on chart area
    Chart,
    /// Right-clicked on a watchlist row
    Watchlist,
}

// ── ADR-094: Sparkline helper ──────────────────────────────────────

/// Render a tiny sparkline (polyline) in a given rect.
pub(crate) fn draw_sparkline(
    painter: &egui::Painter,
    rect: egui::Rect,
    values: &[f64],
    color: egui::Color32,
) {
    if values.len() < 2 {
        return;
    }
    let min = values.iter().copied().fold(f64::MAX, f64::min);
    let max = values.iter().copied().fold(f64::MIN, f64::max);
    let range = (max - min).max(f64::EPSILON);
    let n = values.len();
    let points: Vec<egui::Pos2> = values
        .iter()
        .enumerate()
        .map(|(i, &v)| {
            let x = rect.min.x + (i as f32 / (n - 1) as f32) * rect.width();
            let y = rect.max.y - ((v - min) as f32 / range as f32) * rect.height();
            egui::pos2(x, y)
        })
        .collect();
    painter.add(egui::Shape::line(points, egui::Stroke::new(1.0, color)));
}
