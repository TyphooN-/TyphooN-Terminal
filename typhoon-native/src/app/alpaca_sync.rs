//! Alpaca-specific sync configuration: RPM presets, capacity tiers, no-data
//! persistence, and the bar-count target used by Alpaca's "as much history as
//! the broker has" policy. Broker-agnostic scheduler primitives (candidates,
//! bucket classification, focus-vs-background ordering, the rotating ring) all
//! live in [`super::sync_workset`].

use super::normalize_market_data_symbol;
#[cfg(test)]
use super::sync_workset::alpaca_sync_period_secs;
use super::sync_workset::normalize_sync_timeframe_key;
use std::collections::HashMap;

pub(super) const ALPACA_DEFAULT_HISTORICAL_RPM: u32 =
    typhoon_engine::broker::alpaca::DEFAULT_BAR_REQUESTS_PER_MINUTE;
pub(super) const ALPACA_HISTORICAL_RPM_PRESETS: [(&str, u32); 6] = [
    ("Auto (headers/default 200)", 0),
    ("Basic (200/min)", 200),
    ("Broker Standard (1,000/min)", 1_000),
    ("Broker Plus 3000 (3,000/min)", 3_000),
    ("Broker Plus 5000 (5,000/min)", 5_000),
    ("Algo Trader Plus (10,000/min)", 10_000),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AlpacaSyncCapacity {
    pub fetch_permits: usize,
    pub queue_window: usize,
    pub batch_size: usize,
    pub foreground_reserve: usize,
}

/// Definitive "Alpaca has no bars for this symbol/timeframe" marker.
/// Persisted as JSON under KV key `alpaca:no_data_pairs` so automated sync
/// stops re-requesting pairs the broker never serves.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct AlpacaNoDataPair {
    pub symbol: String,
    pub timeframe: String,
    #[serde(default)]
    pub marked_at: i64,
    #[serde(default)]
    pub reason: String,
}

pub(super) fn alpaca_historical_rpm_hint_label(rpm_hint: u32) -> &'static str {
    ALPACA_HISTORICAL_RPM_PRESETS
        .iter()
        .find_map(|(label, rpm)| (*rpm == rpm_hint).then_some(*label))
        .unwrap_or("Custom")
}

pub(super) fn alpaca_effective_historical_rpm(rpm_hint: u32, observed_rpm: u32) -> u32 {
    if observed_rpm > 0 {
        observed_rpm
    } else if rpm_hint > 0 {
        rpm_hint
    } else {
        ALPACA_DEFAULT_HISTORICAL_RPM
    }
}

pub(super) fn alpaca_sync_capacity_for_rpm(rpm: u32) -> AlpacaSyncCapacity {
    match rpm {
        0..=300 => AlpacaSyncCapacity {
            // Basic/API-header-default Alpaca still has enough historical
            // budget for broad assist sync when we use the multi-symbol stock
            // bars endpoint. Keep permits bounded by the detected tier, but
            // feed the scheduler enough symbols to create useful batch calls
            // instead of dribbling 6 symbols per tick across a 12k catalog.
            fetch_permits: 8,
            queue_window: 64,
            batch_size: 32,
            foreground_reserve: 4,
        },
        301..=1_500 => AlpacaSyncCapacity {
            fetch_permits: 8,
            queue_window: 96,
            batch_size: 48,
            foreground_reserve: 4,
        },
        1_501..=4_000 => AlpacaSyncCapacity {
            fetch_permits: 10,
            queue_window: 160,
            batch_size: 80,
            foreground_reserve: 4,
        },
        4_001..=7_000 => AlpacaSyncCapacity {
            fetch_permits: 12,
            queue_window: 240,
            batch_size: 120,
            foreground_reserve: 6,
        },
        _ => AlpacaSyncCapacity {
            fetch_permits: 16,
            queue_window: 320,
            batch_size: 160,
            foreground_reserve: 8,
        },
    }
}

pub(super) fn alpaca_sync_target_bars(tf: &str) -> Option<u32> {
    match normalize_sync_timeframe_key(tf)? {
        "1Min" | "5Min" => None,
        // Alpaca's stock bars API documents native Month aggregations
        // (`[1,2,3,4,6,12]Month`), so 1Month is provider-native here.
        _ => Some(u32::MAX),
    }
}

#[cfg(test)]
fn alpaca_incremental_fetch_limit_at(
    now_s: i64,
    timeframe: &str,
    after_timestamp: Option<&str>,
) -> u32 {
    let Some(after_ts) = after_timestamp else {
        return 1000;
    };
    let Some(period_s) = alpaca_sync_period_secs(timeframe) else {
        return 1000;
    };
    let parsed = match chrono::DateTime::parse_from_rfc3339(after_ts) {
        Ok(dt) => dt.with_timezone(&chrono::Utc),
        Err(_) => return 1000,
    };
    let age_s = now_s.saturating_sub(parsed.timestamp()).max(0);
    let gap_bars = ((age_s + period_s - 1) / period_s).max(1) as u32;
    let headroom = (gap_bars / 2).max(8);
    gap_bars.saturating_add(headroom).clamp(32, 1000)
}

pub(super) fn deserialize_alpaca_no_data_pairs(json: &str) -> Option<Vec<AlpacaNoDataPair>> {
    if let Ok(entries) = serde_json::from_str::<Vec<AlpacaNoDataPair>>(json) {
        return Some(entries);
    }
    if let Ok(entries) = serde_json::from_str::<HashMap<String, AlpacaNoDataPair>>(json) {
        return Some(entries.into_values().collect());
    }
    if let Ok(entries) = serde_json::from_str::<HashMap<String, i64>>(json) {
        return Some(
            entries
                .into_iter()
                .filter_map(|(key, marked_at)| {
                    let (symbol, timeframe) = key.split_once(':')?;
                    Some(AlpacaNoDataPair {
                        symbol: normalize_market_data_symbol(symbol).replace('/', ""),
                        timeframe: normalize_sync_timeframe_key(timeframe)
                            .unwrap_or(timeframe)
                            .to_string(),
                        marked_at,
                        reason: "legacy persisted no-data mark".to_string(),
                    })
                })
                .collect(),
        );
    }
    if let Ok(entries) = serde_json::from_str::<Vec<String>>(json) {
        return Some(
            entries
                .into_iter()
                .filter_map(|key| {
                    let (symbol, timeframe) = key.split_once(':')?;
                    Some(AlpacaNoDataPair {
                        symbol: normalize_market_data_symbol(symbol).replace('/', ""),
                        timeframe: normalize_sync_timeframe_key(timeframe)
                            .unwrap_or(timeframe)
                            .to_string(),
                        marked_at: 0,
                        reason: "legacy persisted no-data mark".to_string(),
                    })
                })
                .collect(),
        );
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpaca_effective_historical_rpm_prefers_observed_limit() {
        assert_eq!(alpaca_effective_historical_rpm(0, 0), 200);
        assert_eq!(alpaca_effective_historical_rpm(1_000, 0), 1_000);
        assert_eq!(alpaca_effective_historical_rpm(1_000, 10_000), 10_000);
    }

    #[test]
    fn alpaca_sync_capacity_scales_up_for_higher_rpm() {
        let basic = alpaca_sync_capacity_for_rpm(200);
        let plus = alpaca_sync_capacity_for_rpm(10_000);

        assert!(plus.fetch_permits > basic.fetch_permits);
        assert!(plus.queue_window > basic.queue_window);
        assert!(plus.batch_size > basic.batch_size);
    }

    #[test]
    fn basic_alpaca_capacity_feeds_batch_endpoint_for_broad_assist_sync() {
        let basic = alpaca_sync_capacity_for_rpm(200);

        assert_eq!(basic.fetch_permits, 8);
        assert_eq!(basic.queue_window, 64);
        assert_eq!(basic.batch_size, 32);
        assert!(
            basic.batch_size > basic.fetch_permits,
            "scheduler should feed multi-symbol batch calls, not one symbol per worker"
        );
    }

    #[test]
    fn alpaca_capacity_is_monotonic_across_detected_tiers() {
        let tiers = [200, 1_000, 3_000, 5_000, 10_000];
        let mut previous = alpaca_sync_capacity_for_rpm(tiers[0]);
        for rpm in tiers.into_iter().skip(1) {
            let next = alpaca_sync_capacity_for_rpm(rpm);
            assert!(next.fetch_permits >= previous.fetch_permits, "rpm={rpm}");
            assert!(next.queue_window >= previous.queue_window, "rpm={rpm}");
            assert!(next.batch_size >= previous.batch_size, "rpm={rpm}");
            previous = next;
        }
    }

    #[test]
    fn alpaca_historical_rpm_hint_label_recognizes_known_presets_and_falls_back_to_custom() {
        assert_eq!(
            alpaca_historical_rpm_hint_label(0),
            "Auto (headers/default 200)"
        );
        assert_eq!(alpaca_historical_rpm_hint_label(200), "Basic (200/min)");
        assert_eq!(alpaca_historical_rpm_hint_label(12_345), "Custom");
    }

    #[test]
    fn alpaca_sync_target_bars_returns_max_for_supported_timeframes() {
        assert!(alpaca_sync_target_bars("1Min").is_none());
        assert!(alpaca_sync_target_bars("5Min").is_none());
        assert_eq!(alpaca_sync_target_bars("15Min"), Some(u32::MAX));
        assert_eq!(alpaca_sync_target_bars("MN1"), Some(u32::MAX));
        assert!(alpaca_sync_target_bars("bogus").is_none());
    }

    #[test]
    fn deserialize_alpaca_no_data_pairs_accepts_legacy_string_keys() {
        let entries = deserialize_alpaca_no_data_pairs("[\"AAGIY:1Hour\",\"FNGR:1Day\"]")
            .expect("legacy string-key format should deserialize");

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].symbol, "AAGIY");
        assert_eq!(entries[0].timeframe, "1Hour");
        assert_eq!(entries[1].symbol, "FNGR");
        assert_eq!(entries[1].timeframe, "1Day");
    }

    #[test]
    fn deserialize_alpaca_no_data_pairs_accepts_legacy_timestamp_map() {
        let entries = deserialize_alpaca_no_data_pairs("{\"AAGIY:1Hour\":1700000000}")
            .expect("legacy timestamp-map format should deserialize");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].symbol, "AAGIY");
        assert_eq!(entries[0].timeframe, "1Hour");
        assert_eq!(entries[0].marked_at, 1_700_000_000);
    }

    #[test]
    fn deserialize_alpaca_no_data_pairs_accepts_object_map() {
        let json = r#"{"AAGIY:1Hour":{"symbol":"AAGIY","timeframe":"1Hour","marked_at":7,"reason":"no bars"}}"#;
        let entries =
            deserialize_alpaca_no_data_pairs(json).expect("object-map format should deserialize");

        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].marked_at, 7);
        assert_eq!(entries[0].reason, "no bars");
    }

    #[test]
    fn alpaca_incremental_fetch_limit_scales_with_small_gap() {
        let now_s = 1_700_000_000i64;
        let recent = "2023-11-14T22:12:20Z";
        let older = "2023-11-14T16:13:20Z";

        let recent_limit = alpaca_incremental_fetch_limit_at(now_s, "1Min", Some(recent));
        let older_limit = alpaca_incremental_fetch_limit_at(now_s, "1Min", Some(older));

        assert_eq!(recent_limit, 32);
        assert!(older_limit > recent_limit);
        assert!(older_limit < 1000);
    }

    #[test]
    fn alpaca_incremental_fetch_limit_invalid_timestamp_falls_back_to_max() {
        assert_eq!(
            alpaca_incremental_fetch_limit_at(1_700_000_000, "1Min", Some("not-rfc3339")),
            1000
        );
    }
}
