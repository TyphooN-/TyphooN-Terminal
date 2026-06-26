/// Watchlist row data (TradingView-style).
#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub(crate) struct KrakenEquityQuoteMeta {
    pub(crate) received_at_ms: i64,
    pub(crate) quote_time_ms: i64,
    pub(crate) delayed: bool,
    pub(crate) price: f64,
}

// `WatchlistRow` and its cache-row builders now live in typhoon-engine (ADR-127 / ADR-125
// Target 3 prep) so broker-runtime code can use them without depending on native.
// Re-exported here so native call sites via the `state` glob are unchanged.
pub(crate) use typhoon_engine::core::watchlist::{WatchlistRow, watchlist_row_from_raw_bars};
