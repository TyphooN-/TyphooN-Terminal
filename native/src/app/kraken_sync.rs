//! Kraken-specific sync-target shape.
//!
//! Kraken's public OHLC endpoint is a provider-window API — each request
//! returns the most recent ~720 bars per interval (monthly is shorter in
//! practice). The Kraken internal equities history endpoint behaves the same
//! way but with variable per-symbol depth, so a fixed target would cause
//! short-listed equities to look permanently under-filled. Kraken Futures, by
//! contrast, exposes deep history and can absorb the same "ask for everything"
//! target used by Alpaca and Tastytrade.

use super::sync_workset::normalize_sync_timeframe_key;
use super::{KRAKEN_SPOT_MONTH_PROVIDER_WINDOW_BARS, KRAKEN_SPOT_PROVIDER_WINDOW_BARS};

pub(super) fn kraken_sync_target_bars(tf: &str) -> Option<u32> {
    match normalize_sync_timeframe_key(tf)? {
        "1Min" | "5Min" | "15Min" | "30Min" | "1Hour" | "4Hour" | "1Day" | "1Week" => {
            Some(KRAKEN_SPOT_PROVIDER_WINDOW_BARS)
        }
        "1Month" => Some(KRAKEN_SPOT_MONTH_PROVIDER_WINDOW_BARS),
        _ => None,
    }
}

pub(super) fn kraken_equities_sync_target_bars(tf: &str) -> Option<u32> {
    // Kraken internal equities history returns the provider's available window.
    // Many tokenized equities have short listing histories, so a fixed depth
    // target makes valid caches look permanently under-filled and causes the
    // scheduler to refetch the same WOK/TNDM windows every pass.
    normalize_sync_timeframe_key(tf)?;
    None
}

pub(super) fn kraken_futures_sync_target_bars(tf: &str) -> Option<u32> {
    normalize_sync_timeframe_key(tf).map(|_| u32::MAX)
}

#[cfg(test)]
mod tests {
    use super::super::sync_workset::{
        AlpacaBackfillCompletePair, SyncCacheState, alpaca_fetch_key,
        select_alpaca_sync_candidates,
    };
    use super::*;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn kraken_spot_target_uses_provider_window_for_intraday_through_weekly() {
        assert_eq!(
            kraken_sync_target_bars("1Min"),
            Some(KRAKEN_SPOT_PROVIDER_WINDOW_BARS)
        );
        assert_eq!(
            kraken_sync_target_bars("1Week"),
            Some(KRAKEN_SPOT_PROVIDER_WINDOW_BARS)
        );
        assert_eq!(
            kraken_sync_target_bars("1Month"),
            Some(KRAKEN_SPOT_MONTH_PROVIDER_WINDOW_BARS)
        );
        assert!(kraken_sync_target_bars("bogus").is_none());
    }

    #[test]
    fn kraken_equities_target_returns_none_so_short_history_is_not_treated_as_backfill_gap() {
        assert!(kraken_equities_sync_target_bars("1Day").is_none());
        assert!(kraken_equities_sync_target_bars("M1").is_none());
        assert!(kraken_equities_sync_target_bars("bogus").is_none());
    }

    #[test]
    fn kraken_futures_targets_request_full_history() {
        assert_eq!(kraken_futures_sync_target_bars("1Min"), Some(u32::MAX));
        assert_eq!(kraken_futures_sync_target_bars("1Month"), Some(u32::MAX));
        assert!(kraken_futures_sync_target_bars("bogus").is_none());
    }

    #[test]
    fn kraken_provider_window_target_does_not_force_permanent_backfill() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["BTCUSD".to_string()];
        let timeframes = vec!["1Min".to_string()];
        let state_map = HashMap::from([(
            ("BTCUSD".to_string(), "1Min".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 60,
                write_ts_s: now_s - 60,
                bar_count: KRAKEN_SPOT_PROVIDER_WINDOW_BARS as i64,
            },
        )]);

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &state_map,
            &HashSet::new(),
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            1,
            now_s,
            kraken_sync_target_bars,
        );

        assert!(selected.is_empty());
    }

    #[test]
    fn kraken_equities_short_provider_history_does_not_repeat_backfill() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["WOK".to_string()];
        let timeframes = vec!["1Month".to_string(), "1Week".to_string()];
        let state_map = HashMap::from([
            (
                ("WOK".to_string(), "1Month".to_string()),
                SyncCacheState {
                    last_bar_ts_s: now_s - 60,
                    write_ts_s: now_s - 60,
                    bar_count: 14,
                },
            ),
            (
                ("WOK".to_string(), "1Week".to_string()),
                SyncCacheState {
                    last_bar_ts_s: now_s - 60,
                    write_ts_s: now_s - 60,
                    bar_count: 63,
                },
            ),
        ]);

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &state_map,
            &HashSet::new(),
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            4,
            now_s,
            kraken_equities_sync_target_bars,
        );

        assert!(
            selected.is_empty(),
            "short but fresh Kraken equities histories are complete provider windows, not backfill gaps"
        );
    }

    #[test]
    fn kraken_futures_full_history_marker_suppresses_repeat_backfill() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["PI_XBTUSD".to_string()];
        let timeframes = vec!["1Hour".to_string()];
        let state_map = HashMap::from([(
            ("PI_XBTUSD".to_string(), "1Hour".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 3600,
                write_ts_s: now_s - 60,
                bar_count: 50_000,
            },
        )]);
        let backfill_complete = HashMap::from([(
            alpaca_fetch_key("PI_XBTUSD", "1Hour"),
            AlpacaBackfillCompletePair {
                symbol: "PI_XBTUSD".to_string(),
                timeframe: "1Hour".to_string(),
                marked_at: now_s,
                bar_count: 50_000,
                target_bars: 50_000,
            },
        )]);

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &state_map,
            &HashSet::new(),
            &HashSet::new(),
            &backfill_complete,
            &HashSet::new(),
            1,
            now_s,
            kraken_futures_sync_target_bars,
        );

        assert!(selected.is_empty());
    }
}
