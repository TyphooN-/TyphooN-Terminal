//! Broker market-data sync budgets and small timeframe helpers.
//!
//! Kept out of `app.rs` so scheduler policy has a small, compile-checkable home
//! instead of adding more constants and helper code to the main application unit.

pub(super) const KRAKEN_PUBLIC_FETCH_PERMITS: usize = 24;
pub(super) const KRAKEN_SPOT_QUEUE_WINDOW: usize = 240;
pub(super) const KRAKEN_FUTURES_QUEUE_WINDOW: usize = 144;
pub(super) const ALPACA_BACKGROUND_SCAN_LIMIT: usize = 768;
pub(super) const KRAKEN_SPOT_BACKGROUND_SCAN_LIMIT: usize = 768;
pub(super) const KRAKEN_FUTURES_BACKGROUND_SCAN_LIMIT: usize = 384;
pub(super) const TASTYTRADE_BACKGROUND_SCAN_LIMIT: usize = 192;

/// AC/desktop full-tilt mode keeps request pressure high enough to saturate API
/// allowances and async worker capacity. It is still bounded: pending sets,
/// provider rate limiters, no-data tombstones, and backfill-complete markers stay
/// in force so we do not turn a large universe into duplicate request storms.
pub(super) const FULL_TILT_SYNC_INTERVAL_SECS: u64 = 1;
pub(super) const BALANCED_SYNC_INTERVAL_SECS: u64 = 60;
pub(super) const ALPACA_FULL_TILT_QUEUE_WINDOW: usize = 160;
pub(super) const ALPACA_FULL_TILT_BATCH_SIZE: usize = 120;
pub(super) const ALPACA_FULL_TILT_FETCH_PERMITS: usize = 8;
pub(super) const ALPACA_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 4_096;
pub(super) const KRAKEN_SPOT_FULL_TILT_QUEUE_WINDOW: usize = 256;
pub(super) const KRAKEN_SPOT_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 2_048;
pub(super) const KRAKEN_EQUITIES_FULL_TILT_QUEUE_WINDOW: usize = 512;
pub(super) const KRAKEN_EQUITIES_FULL_TILT_BATCH_SIZE: usize = 192;
pub(super) const KRAKEN_EQUITIES_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 8192;
/// Concurrent in-flight iapi equity-history fetches. The engine-side
/// `iapi_limiter` token bucket is the real rate governor (≤ `aimd_max_rate`
/// req/s); this only needs to be high enough that the bucket — not a tiny
/// semaphore — is the binding constraint. Size it to the AIMD ceiling × typical
/// round-trip: ~120 req/s × ~0.4 s ≈ 48 concurrent. Beyond that the bucket
/// paces dispatch anyway, so extra permits just park on the token wait.
/// (Was 2, which capped throughput at ~6 req/s regardless of the bucket and
/// left the AIMD ramp probing a ceiling it could never actually drive.)
pub(super) const KRAKEN_EQUITIES_FETCH_PERMITS: usize = 48;
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
pub(super) const KRAKEN_FUTURES_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 1024;
pub(super) const TASTYTRADE_FULL_TILT_QUEUE_WINDOW: usize = 64;
pub(super) const TASTYTRADE_FULL_TILT_BATCH_SIZE: usize = 32;
pub(super) const TASTYTRADE_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 1024;

pub(super) const YAHOO_CHART_QUEUE_WINDOW: usize = 12;
pub(super) const YAHOO_CHART_BATCH_SIZE: usize = 1;
pub(super) const YAHOO_CHART_FULL_TILT_QUEUE_WINDOW: usize = 72;
pub(super) const YAHOO_CHART_FULL_TILT_BATCH_SIZE: usize = 6;
pub(super) const YAHOO_CHART_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 2_048;

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

/// Kraken Spot public OHLC accepts daily/weekly provider intervals but has no
/// true calendar-month bar. Any monthly Kraken view must be constructed from
/// cached daily bars on the merged/chart path, never stored as `kraken:*:1Month`
/// provider-native data.
pub(super) fn kraken_spot_native_timeframe(tf: &str) -> bool {
    matches!(
        tf,
        "1Min" | "5Min" | "15Min" | "30Min" | "1Hour" | "4Hour" | "1Day" | "1Week"
    )
}

/// Kraken Equities/xStocks is WS-first for live/current OHLC. M1/M5 are valid
/// for real tokenized xStocks now, and M15/M30/H1/H4/D1/W1 remain visible for
/// the native xStock demand set. Monthly is intentionally excluded: construct
/// it from D1 on the merged/chart path instead of writing
/// `kraken-equities:*:1Month` KVs.
pub(super) fn kraken_equity_full_universe_timeframe(tf: &str) -> bool {
    matches!(
        tf,
        "1Min" | "5Min" | "15Min" | "30Min" | "1Hour" | "4Hour" | "1Day" | "1Week"
    )
}

/// Broad Kraken-equity coverage target for non-native assist sources. The sync
/// goal is full-catalog, high-timeframe-first convergence across every enabled
/// timeframe/source: MN1 -> W1 -> D1 -> H4 -> H1 -> M30 -> M15 -> M5 -> M1.
/// Provider windows still cap how deep low-timeframe history can go, but they
/// should not keep low-timeframe rows permanently demand-scoped when full sync
/// is explicitly enabled.
pub(super) fn kraken_equity_broad_fallback_timeframe(tf: &str) -> bool {
    matches!(
        tf,
        "1Min" | "5Min" | "15Min" | "30Min" | "1Hour" | "4Hour" | "1Day" | "1Week" | "1Month"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_tilt_profile_refills_every_tick_with_responsive_bounded_windows() {
        assert_eq!(FULL_TILT_SYNC_INTERVAL_SECS, 1);
        assert!(ALPACA_FULL_TILT_QUEUE_WINDOW >= 64);
        assert!(ALPACA_FULL_TILT_BATCH_SIZE >= 32);
        assert!(ALPACA_FULL_TILT_FETCH_PERMITS >= 8);
        assert!(YAHOO_CHART_FULL_TILT_QUEUE_WINDOW >= 72);
        assert!(YAHOO_CHART_FULL_TILT_BATCH_SIZE >= 6);
        assert!(YAHOO_CHART_FULL_TILT_BACKGROUND_SCAN_LIMIT >= 2_048);
        assert!(KRAKEN_SPOT_FULL_TILT_QUEUE_WINDOW > KRAKEN_SPOT_QUEUE_WINDOW);
        assert!(KRAKEN_FUTURES_FULL_TILT_QUEUE_WINDOW > KRAKEN_FUTURES_QUEUE_WINDOW);
        assert!(TASTYTRADE_FULL_TILT_QUEUE_WINDOW >= 64);
        assert_eq!(KRAKEN_PUBLIC_FETCH_PERMITS, 24);
    }

    #[test]
    fn kraken_rest_provider_window_stays_within_public_ohlc_ceiling() {
        assert_eq!(KRAKEN_SPOT_PROVIDER_WINDOW_BARS, 720);
        assert!(kraken_spot_native_timeframe("1Week"));
        assert!(!kraken_spot_native_timeframe("1Month"));
    }

    #[test]
    fn kraken_equity_full_universe_sync_is_native_through_weekly() {
        assert!(kraken_equity_full_universe_timeframe("1Min"));
        assert!(kraken_equity_full_universe_timeframe("5Min"));
        assert!(kraken_equity_full_universe_timeframe("15Min"));
        assert!(kraken_equity_full_universe_timeframe("30Min"));
        assert!(kraken_equity_full_universe_timeframe("1Hour"));
        assert!(kraken_equity_full_universe_timeframe("4Hour"));
        assert!(kraken_equity_full_universe_timeframe("1Day"));
        assert!(kraken_equity_full_universe_timeframe("1Week"));
        assert!(!kraken_equity_full_universe_timeframe("1Month"));
    }

    #[test]
    fn kraken_equity_broad_fallback_sync_covers_all_standard_timeframes() {
        assert!(kraken_equity_broad_fallback_timeframe("1Min"));
        assert!(kraken_equity_broad_fallback_timeframe("5Min"));
        assert!(kraken_equity_broad_fallback_timeframe("15Min"));
        assert!(kraken_equity_broad_fallback_timeframe("4Hour"));
        assert!(kraken_equity_broad_fallback_timeframe("1Day"));
    }
}
