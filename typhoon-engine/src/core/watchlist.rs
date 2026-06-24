//! Watchlist row DTO.
//!
//! `WatchlistRow` is the per-symbol quote row shown in the watchlist panel and carried
//! by the broker message protocol (`BrokerMsg::WatchlistQuotes`). It lives in the engine
//! (ADR-127) so the protocol depends only on engine/std — a pure `serde` data type, no UI
//! or runtime coupling. The native side keeps the row *builders*
//! (`watchlist_row_from_raw_bars`, `empty_watchlist_row`) and re-exports this struct.

use serde::{Deserialize, Serialize};

/// Watchlist row data (TradingView-style).
#[derive(Clone, Serialize, Deserialize)]
pub struct WatchlistRow {
    /// Display symbol name (e.g. "BTCUSD", "SLV", "CC").
    pub symbol: String,
    /// Full cache key for loading.
    pub cache_key: String,
    /// Last close price.
    pub last: f64,
    /// Previous close (for change calculation).
    pub prev_close: f64,
    /// Current-day regular-session close (authoritative daily close, e.g.
    /// Alpaca `dailyBar.c` / Yahoo `regularMarketPrice`). Timeframe-independent,
    /// unlike a chart's own last-bar close, which differs between H1/H4/W1.
    /// `0.0` when unknown. Used to drive the extended-hours "Daily Close" badge.
    #[serde(default)]
    pub regular_close: f64,
    /// Absolute change.
    pub change: f64,
    /// Percentage change.
    pub change_pct: f64,
    /// Last bar volume.
    pub volume: f64,
    /// Extended hours change % (pre/post market).
    pub ext_change_pct: f64,
    /// Live bid from WS (0.0 when none or stale >30s).
    #[serde(default, skip)]
    pub live_bid: f64,
    /// Live ask from WS (0.0 when none or stale >30s).
    #[serde(default, skip)]
    pub live_ask: f64,
    /// When the live quote arrived (for freshness check, same rule as charts).
    #[serde(default, skip)]
    pub live_quote_at: Option<std::time::Instant>,
}
