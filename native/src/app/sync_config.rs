//! Broker market-data sync budgets and small timeframe helpers.
//!
//! Kept out of `app.rs` so scheduler policy has a small, compile-checkable home
//! instead of adding more constants and helper code to the main application unit.

pub(super) const KRAKEN_PUBLIC_FETCH_PERMITS: usize = 24;
pub(super) const KRAKEN_SPOT_QUEUE_WINDOW: usize = 160;
pub(super) const KRAKEN_FUTURES_QUEUE_WINDOW: usize = 96;
pub(super) const ALPACA_BACKGROUND_SCAN_LIMIT: usize = 384;
pub(super) const KRAKEN_SPOT_BACKGROUND_SCAN_LIMIT: usize = 384;
pub(super) const KRAKEN_FUTURES_BACKGROUND_SCAN_LIMIT: usize = 192;
pub(super) const TASTYTRADE_BACKGROUND_SCAN_LIMIT: usize = 96;

/// AC/desktop full-tilt mode keeps request pressure high enough to saturate API
/// allowances and async worker capacity. It is still bounded: pending sets,
/// provider rate limiters, no-data tombstones, and backfill-complete markers stay
/// in force so we do not turn a large universe into duplicate request storms.
pub(super) const FULL_TILT_SYNC_INTERVAL_SECS: u64 = 1;
pub(super) const BALANCED_SYNC_INTERVAL_SECS: u64 = 60;
pub(super) const ALPACA_FULL_TILT_QUEUE_WINDOW: usize = 256;
pub(super) const ALPACA_FULL_TILT_BATCH_SIZE: usize = 192;
pub(super) const ALPACA_FULL_TILT_FETCH_PERMITS: usize = 64;
pub(super) const ALPACA_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 16_384;
pub(super) const KRAKEN_SPOT_FULL_TILT_QUEUE_WINDOW: usize = 640;
pub(super) const KRAKEN_SPOT_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 16_384;
pub(super) const KRAKEN_EQUITIES_FULL_TILT_QUEUE_WINDOW: usize = 96;
pub(super) const KRAKEN_EQUITIES_FULL_TILT_BATCH_SIZE: usize = 48;
pub(super) const KRAKEN_EQUITIES_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 16_384;
// Per-call iapi spacing (was KRAKEN_EQUITIES_HISTORY_MIN_INTERVAL_MS) and the
// flat post-429 pause (was KRAKEN_EQUITIES_HISTORY_429_BACKOFF_SECS) are now
// owned by the engine-side `iapi_limiter` (token bucket + escalating
// exponential backoff, persisted across restarts).
/// Minimum interval between full REST `TradesHistory` fetches issued by the
/// periodic KrakenBalances handler. The `ownTrades` WebSocket already keeps
/// the trade list current; the REST pull is a safety-net resync, not a
/// primary feed.
pub(super) const KRAKEN_TRADES_REST_REFRESH_SECS: u64 = 600;
pub(super) const KRAKEN_FUTURES_FULL_TILT_QUEUE_WINDOW: usize = 384;
pub(super) const KRAKEN_FUTURES_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 8192;
pub(super) const TASTYTRADE_FULL_TILT_QUEUE_WINDOW: usize = 192;
pub(super) const TASTYTRADE_FULL_TILT_BATCH_SIZE: usize = 96;
pub(super) const TASTYTRADE_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 8192;

/// Largest `MAX_BARS` value that can safely cross the MT5 demand.txt / MQL5
/// boundary. This is a provider-maximum sentinel, not a local history target:
/// the terminal asks the EA for everything the broker server can provide, then
/// the saturation memory suppresses repeat full requests once the count stops
/// growing for a symbol/timeframe.
pub(super) const MT5_PROVIDER_MAX_BARS: u32 = i32::MAX as u32;

/// Kraken Spot public OHLC is a provider-window API, not a traversal API. Kraken
/// documents the endpoint as returning the most recent ~720 candles per interval
/// (monthly is shorter in practice), so these values are external provider
/// windows rather than terminal-side depth caps.
pub(super) const KRAKEN_SPOT_PROVIDER_WINDOW_BARS: u32 = 720;
pub(super) const KRAKEN_SPOT_MONTH_PROVIDER_WINDOW_BARS: u32 = 24;

/// Kraken Securities/iapi is the bottleneck in full-universe sync: the safe
/// discovered ceiling is often below 1 req/s, and history is one symbol × one
/// timeframe per request. Syncing ~12.6k equities across every intraday TF would
/// take days and starve charts/owned positions. The broad universe lane therefore
/// keeps only the durable higher-TF bars current; focused/owned/chart-triggered
/// fetches can still request 15Min/30Min/1Hour/4Hour through
/// `queue_kraken_equity_fetch`.
pub(super) fn kraken_equity_full_universe_timeframe(tf: &str) -> bool {
    matches!(tf, "1Day" | "1Week" | "1Month")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_tilt_profile_refills_every_tick_with_larger_safe_windows() {
        assert_eq!(FULL_TILT_SYNC_INTERVAL_SECS, 1);
        assert!(ALPACA_FULL_TILT_QUEUE_WINDOW >= 256);
        assert!(ALPACA_FULL_TILT_BATCH_SIZE >= 192);
        assert!(ALPACA_FULL_TILT_FETCH_PERMITS >= 64);
        assert!(KRAKEN_SPOT_FULL_TILT_QUEUE_WINDOW >= KRAKEN_SPOT_QUEUE_WINDOW * 4);
        assert!(KRAKEN_FUTURES_FULL_TILT_QUEUE_WINDOW >= KRAKEN_FUTURES_QUEUE_WINDOW * 4);
        assert!(TASTYTRADE_FULL_TILT_QUEUE_WINDOW >= 192);
    }

    #[test]
    fn kraken_rest_provider_window_stays_within_public_ohlc_ceiling() {
        assert_eq!(KRAKEN_SPOT_PROVIDER_WINDOW_BARS, 720);
        assert!(KRAKEN_SPOT_MONTH_PROVIDER_WINDOW_BARS < KRAKEN_SPOT_PROVIDER_WINDOW_BARS);
    }

    #[test]
    fn kraken_equity_full_universe_sync_is_high_timeframe_only() {
        assert!(!kraken_equity_full_universe_timeframe("15Min"));
        assert!(!kraken_equity_full_universe_timeframe("4Hour"));
        assert!(kraken_equity_full_universe_timeframe("1Day"));
        assert!(kraken_equity_full_universe_timeframe("1Week"));
        assert!(kraken_equity_full_universe_timeframe("1Month"));
    }
}
