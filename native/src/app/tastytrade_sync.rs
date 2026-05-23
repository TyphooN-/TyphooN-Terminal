//! Tastytrade-specific sync-target shape and history floor.
//!
//! Tastytrade serves bars via DxLink, which can return many years of history
//! per request, so the scheduler should ask for everything the broker has —
//! the backfill-complete marker handles the steady-state "done" signal once
//! the broker stops returning new pre-window bars.

use super::sync_workset::normalize_sync_timeframe_key;

pub(super) fn tastytrade_sync_target_bars(tf: &str) -> Option<u32> {
    normalize_sync_timeframe_key(tf).map(|_| u32::MAX)
}

/// Earliest history floor Tastytrade backfill should ever ask for.
///
/// Tastytrade's DxLink history endpoint accepts any timestamp but the broker
/// has no usable equity bars before ~2000. Capping the request at the start
/// of 2000 keeps DxLink responses bounded without affecting any reachable
/// pre-window history.
pub(super) fn tastytrade_earliest_history_ms() -> i64 {
    chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|ndt| ndt.and_utc().timestamp_millis())
        .unwrap_or(0)
}

pub(super) fn tastytrade_initial_from_time_ms(_timeframe: &str, _now_ms: i64) -> i64 {
    tastytrade_earliest_history_ms()
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
    fn tastytrade_targets_request_full_dxlink_history() {
        assert_eq!(tastytrade_sync_target_bars("1Min"), Some(u32::MAX));
        assert_eq!(tastytrade_sync_target_bars("4Hour"), Some(u32::MAX));
        assert_eq!(tastytrade_sync_target_bars("1Day"), Some(u32::MAX));
        assert_eq!(tastytrade_sync_target_bars("1Month"), Some(u32::MAX));
        assert!(tastytrade_sync_target_bars("bogus").is_none());
    }

    #[test]
    fn tastytrade_initial_time_is_floor_bounded() {
        assert_eq!(
            tastytrade_initial_from_time_ms("UNKNOWN", 0),
            tastytrade_earliest_history_ms()
        );
        assert_eq!(
            tastytrade_initial_from_time_ms("D1", 0),
            tastytrade_earliest_history_ms()
        );
    }

    #[test]
    fn tastytrade_initial_time_keeps_headroom_when_recent() {
        let now_ms = chrono::NaiveDate::from_ymd_opt(2025, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        let start = tastytrade_initial_from_time_ms("D1", now_ms);
        assert!(start < now_ms);
        assert!(start >= tastytrade_earliest_history_ms());
    }

    #[test]
    fn tastytrade_earliest_history_ms_is_year_2000_floor() {
        let expected = chrono::NaiveDate::from_ymd_opt(2000, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc()
            .timestamp_millis();
        assert_eq!(tastytrade_earliest_history_ms(), expected);
    }

    #[test]
    fn tastytrade_full_history_marker_suppresses_repeat_backfill() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string()];
        let timeframes = vec!["1Hour".to_string()];
        let state_map = HashMap::from([(
            ("AAPL".to_string(), "1Hour".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 3600,
                write_ts_s: now_s - 60,
                bar_count: 37_421,
            },
        )]);
        let backfill_complete = HashMap::from([(
            alpaca_fetch_key("AAPL", "1Hour"),
            AlpacaBackfillCompletePair {
                symbol: "AAPL".to_string(),
                timeframe: "1Hour".to_string(),
                marked_at: now_s,
                bar_count: 37_421,
                target_bars: 37_421,
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
            tastytrade_sync_target_bars,
        );

        assert!(selected.is_empty());
    }
}
