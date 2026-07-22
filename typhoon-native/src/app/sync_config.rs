//! Broker market-data sync budgets and small timeframe helpers.
//!
//! Kept out of `app.rs` so scheduler policy has a small, compile-checkable home
//! instead of adding more constants and helper code to the main application unit.

#[cfg(test)]
pub(super) use typhoon_engine::broker::sync_config::KRAKEN_PUBLIC_FETCH_PERMITS;
pub(super) const KRAKEN_SPOT_QUEUE_WINDOW: usize = 240;
pub(super) const KRAKEN_FUTURES_QUEUE_WINDOW: usize = 144;
pub(super) const ALPACA_BACKGROUND_SCAN_LIMIT: usize = 768;
pub(super) const KRAKEN_SPOT_BACKGROUND_SCAN_LIMIT: usize = 768;
pub(super) const KRAKEN_FUTURES_BACKGROUND_SCAN_LIMIT: usize = 384;

/// AC/desktop full-tilt mode keeps request pressure high enough to saturate API
/// allowances and async worker capacity. It is still bounded: pending sets,
/// provider rate limiters, no-data tombstones, and backfill-complete markers stay
/// in force so we do not turn a large universe into duplicate request storms.
///
/// The dispatch budgets below (queue windows + batch sizes) are deliberately
/// generous so each 1s tick refills the broker queues faster than the workers can
/// drain them — the worker therefore never idles waiting on the scheduler, and the
/// only thing pacing actual requests is the per-provider rate limiter (the real
/// ceiling). Pushing these higher is safe: the limiter still throttles to the
/// provider's allowance, so a bigger queue just buffers more in-flight work, it
/// does not raise the request rate past the limit. Background scan limits are NOT
/// in this set on purpose — they run on the render thread, so they stay bounded to
/// avoid the per-tick `pre_broker` stalls.
pub(super) const FULL_TILT_SYNC_INTERVAL_SECS: u64 = 1;
pub(super) const BALANCED_SYNC_INTERVAL_SECS: u64 = 60;

/// Broad heavy scheduler lanes serviced by `run_broad_dispatch_slice`:
/// Kraken spot/equities universe, Kraken futures, Alpaca rotation. Visible
/// passes run at most one lane per frame; hidden passes run every due lane.
pub(super) const BROAD_DISPATCH_LANES: u8 = 3;
/// Floor between refill-driven runs of the same lane. Settlement-triggered
/// refills matter mostly in balanced mode (the periodic interval is 60s
/// there); under full-tilt the 1s periodic cadence dominates, and this floor
/// keeps a continuous settlement stream from turning every frame into a
/// catalog scan.
pub(super) const BROAD_DISPATCH_REFILL_MIN_SPACING: std::time::Duration =
    std::time::Duration::from_millis(250);
/// Minimum spacing between heavy broad-dispatch lane runs on *visible* passes.
/// Each lane's full-catalog workset selection costs 250-490ms on the render
/// thread (12k-symbol universe), so at full-tilt the 1s periodic interval plus
/// settlement-driven refills otherwise put a scan on nearly every visible
/// frame — the persistent ~300ms `dispatch_ms` stalls in the live log. Sync
/// throughput does not depend on visible cadence (the 256-768 deep queue
/// windows buffer between refills and the per-provider rate limiter is the
/// real ceiling), and hidden overnight passes stay unthrottled, so spacing
/// visible dispatch here restores smooth rendering without slowing catch-up.
/// Actively-viewed symbols stay fresh through the separate chart/watchlist
/// demand fetch paths, which are not gated by this.
pub(super) const VISIBLE_BROAD_DISPATCH_MIN_SPACING: std::time::Duration =
    std::time::Duration::from_secs(2);

pub(super) const ALPACA_FULL_TILT_QUEUE_WINDOW: usize = 256;
pub(super) const ALPACA_FULL_TILT_BATCH_SIZE: usize = 200;
pub(super) const ALPACA_FULL_TILT_FETCH_PERMITS: usize = 8;
// Full-tilt background scan limits (rows examined per scheduler run, NOT
// dispatch budgets): these scans run on the render thread, so each lane must
// fit a frame. At ~15-20µs per examined row the previous 2048-4096 limits put
// the Kraken universe lane at 130-160ms and the Alpaca rotation at ~110ms
// every full-tilt second — the constant 250ms+ frame stalls in the live log.
// Small slices don't cost throughput: the rotating cursor sweeps the full
// catalog across ticks, and during catch-up nearly every scanned row is a
// candidate, so slots fill long before the scan limit binds. The limit only
// binds when mostly caught up — exactly when scanning is pure discovery and a
// ~2min full-sweep latency is fine.
pub(super) const ALPACA_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 1_024;
pub(super) const ALPACA_FULL_TILT_LOW_TF_RESERVE_BATCH: usize = 24;
pub(super) const KRAKEN_SPOT_FULL_TILT_QUEUE_WINDOW: usize = 384;
pub(super) const KRAKEN_SPOT_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 768;
pub(super) const KRAKEN_SPOT_FULL_TILT_LOW_TF_RESERVE_BATCH: usize = 16;
pub(super) const KRAKEN_EQUITIES_FULL_TILT_QUEUE_WINDOW: usize = 768;
pub(super) const KRAKEN_EQUITIES_FULL_TILT_BATCH_SIZE: usize = 320;
pub(super) const KRAKEN_EQUITIES_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 768;
// Kraken Securities/iapi permit limits live behind the broker-runtime resource
// seam now so the broker processor can move without depending on native.
// Per-call iapi spacing (was KRAKEN_EQUITIES_HISTORY_MIN_INTERVAL_MS) and the
// flat post-429 pause (was KRAKEN_EQUITIES_HISTORY_429_BACKOFF_SECS) are now
// owned by the engine-side `iapi_limiter` (token bucket + escalating
// exponential backoff, persisted across restarts).
/// Minimum interval between full REST `TradesHistory` fetches issued by the
/// periodic KrakenBalances handler. The `ownTrades` WebSocket already keeps
/// the trade list current; the REST pull is a safety-net resync, not a
/// primary feed.
pub(super) const KRAKEN_TRADES_REST_REFRESH_SECS: u64 = 600;
pub(super) const KRAKEN_FUTURES_FULL_TILT_QUEUE_WINDOW: usize = 576;
pub(super) const KRAKEN_FUTURES_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 512;

pub(super) const YAHOO_CHART_QUEUE_WINDOW: usize = 12;
pub(super) const YAHOO_CHART_BATCH_SIZE: usize = 1;
pub(super) const YAHOO_CHART_FULL_TILT_QUEUE_WINDOW: usize = 120;
pub(super) const YAHOO_CHART_FULL_TILT_BATCH_SIZE: usize = 12;
pub(super) const YAHOO_CHART_FULL_TILT_BACKGROUND_SCAN_LIMIT: usize = 768;

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
        // Scan limits are render-thread frame budgets, deliberately small;
        // assert the floor that still sweeps the catalog in a few minutes.
        assert!(YAHOO_CHART_FULL_TILT_BACKGROUND_SCAN_LIMIT >= 512);
        assert!(KRAKEN_SPOT_FULL_TILT_QUEUE_WINDOW > KRAKEN_SPOT_QUEUE_WINDOW);
        assert!(KRAKEN_FUTURES_FULL_TILT_QUEUE_WINDOW > KRAKEN_FUTURES_QUEUE_WINDOW);
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
