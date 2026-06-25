//! Watchlist row DTO and cache-row builders.
//!
//! `WatchlistRow` is the per-symbol quote row shown in the watchlist panel and carried
//! by the broker message protocol (`BrokerMsg::WatchlistQuotes`). It lives in the engine
//! (ADR-127) so the protocol depends only on engine/std — a pure `serde` data type, no UI
//! or runtime coupling. The row builders live here too so broker-runtime code can build
//! cache-backed watchlist rows without depending on `typhoon-native`.

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

pub fn watchlist_row_from_raw_bars(
    symbol: &str,
    cache_key: &str,
    raw: &[(i64, f64, f64, f64, f64, f64)],
) -> Option<WatchlistRow> {
    let mut valid = raw.iter().filter(|(ts, o, h, l, c, _v)| {
        *ts > 0
            && *o > 0.0
            && *h > 0.0
            && *l > 0.0
            && *c > 0.0
            && o.is_finite()
            && h.is_finite()
            && l.is_finite()
            && c.is_finite()
            && *h >= *l
    });
    let last_bar = valid.next_back()?;
    let prev_bar = valid.next_back().unwrap_or(last_bar);
    let change = last_bar.4 - prev_bar.4;
    let change_pct = if prev_bar.4 > 0.0 {
        change / prev_bar.4 * 100.0
    } else {
        0.0
    };
    Some(WatchlistRow {
        symbol: symbol.to_string(),
        cache_key: cache_key.to_string(),
        last: last_bar.4,
        prev_close: prev_bar.4,
        // Offline cache fallback has no separate regular-session close.
        regular_close: 0.0,
        change,
        change_pct,
        volume: last_bar.5,
        ext_change_pct: 0.0,
        live_bid: 0.0,
        live_ask: 0.0,
        live_quote_at: None,
    })
}

pub fn empty_watchlist_row(symbol: &str) -> WatchlistRow {
    WatchlistRow {
        symbol: symbol.to_string(),
        cache_key: symbol.to_string(),
        last: 0.0,
        prev_close: 0.0,
        regular_close: 0.0,
        change: 0.0,
        change_pct: 0.0,
        volume: 0.0,
        ext_change_pct: 0.0,
        live_bid: 0.0,
        live_ask: 0.0,
        live_quote_at: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn watchlist_row_from_raw_bars_uses_last_two_valid_closes() {
        let raw = vec![
            (1, 10.0, 12.0, 9.0, 10.0, 100.0),
            (2, 10.0, 13.0, 9.0, 12.0, 200.0),
        ];
        let row = watchlist_row_from_raw_bars("TEST", "alpaca:TEST:1Day", &raw).unwrap();
        assert_eq!(row.symbol, "TEST");
        assert_eq!(row.cache_key, "alpaca:TEST:1Day");
        assert_eq!(row.last, 12.0);
        assert_eq!(row.prev_close, 10.0);
        assert_eq!(row.change, 2.0);
        assert_eq!(row.change_pct, 20.0);
    }

    #[test]
    fn empty_watchlist_row_has_zero_quote_fields() {
        let row = empty_watchlist_row("AAPL");
        assert_eq!(row.symbol, "AAPL");
        assert_eq!(row.cache_key, "AAPL");
        assert_eq!(row.last, 0.0);
        assert_eq!(row.live_bid, 0.0);
        assert!(row.live_quote_at.is_none());
    }
}
