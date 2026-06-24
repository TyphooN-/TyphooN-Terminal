/// Watchlist row data (TradingView-style).
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct KrakenEquityQuoteMeta {
    pub(crate) received_at_ms: i64,
    pub(crate) quote_time_ms: i64,
    pub(crate) delayed: bool,
    pub(crate) price: f64,
}

// `WatchlistRow` now lives in typhoon-engine (ADR-127) so the broker message protocol
// depends only on engine/std. Re-exported here so the row builders below and the native
// call sites (via the `state` glob) are unchanged.
pub(crate) use typhoon_engine::core::watchlist::WatchlistRow;
pub(crate) fn watchlist_row_from_raw_bars(
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

pub(crate) fn empty_watchlist_row(symbol: &str) -> WatchlistRow {
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
