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

/// Kraken Securities/iapi is the native/authoritative lane but provider pressure
/// is empirical, session/IP dependent, and AIMD-controlled. Keep native iapi
/// broad-universe pressure on durable higher-TF bars for now; use provider
/// assist lanes for broad 15Min+ coverage where those sources can supply it.
pub(super) fn kraken_equity_full_universe_timeframe(tf: &str) -> bool {
    matches!(tf, "1Day" | "1Week" | "1Month")
}

/// Broad Kraken-equity coverage target for non-native assist sources. 1Min/5Min
/// are kept demand/focus scoped because provider access/window limits make them
/// freshness assists, not realistic 13k-symbol backlog-fill lanes. 15Min+ should
/// rotate over the whole Kraken equity catalog when Alpaca/Yahoo/Stooq are
/// enabled so Sync Status does not stay permanently bare below D1.
pub(super) fn kraken_equity_broad_fallback_timeframe(tf: &str) -> bool {
    matches!(
        tf,
        "15Min" | "30Min" | "1Hour" | "4Hour" | "1Day" | "1Week" | "1Month"
    )
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
    fn kraken_equity_full_universe_sync_is_native_high_timeframe_only() {
        assert!(!kraken_equity_full_universe_timeframe("15Min"));
        assert!(!kraken_equity_full_universe_timeframe("4Hour"));
        assert!(kraken_equity_full_universe_timeframe("1Day"));
        assert!(kraken_equity_full_universe_timeframe("1Week"));
        assert!(kraken_equity_full_universe_timeframe("1Month"));
    }

    #[test]
    fn kraken_equity_broad_fallback_sync_starts_at_15min() {
        assert!(!kraken_equity_broad_fallback_timeframe("1Min"));
        assert!(!kraken_equity_broad_fallback_timeframe("5Min"));
        assert!(kraken_equity_broad_fallback_timeframe("15Min"));
        assert!(kraken_equity_broad_fallback_timeframe("4Hour"));
        assert!(kraken_equity_broad_fallback_timeframe("1Day"));
    }
}
