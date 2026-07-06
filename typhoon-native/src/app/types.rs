//! Extracted from app.rs: types helpers.

use super::*;

// ─── types ───────────────────────────────────────────────────────────────────

// `Bar` / `ChartType` / `Timeframe` now live in the `typhoon-chart-ui` crate (ADR-125
// Target 2, slice 1) — the chart rendering + state layers are built on them. Re-exported
// here so existing `crate::app::types::{Bar,ChartType,Timeframe}` call sites (and the
// `super::*` glob) resolve unchanged.
pub(crate) use typhoon_chart_ui::types::{Bar, ChartType, Timeframe};

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
        chrono::Utc::now().format("%H:%M:%S").to_string()
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
