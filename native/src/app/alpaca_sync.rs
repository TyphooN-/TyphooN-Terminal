use super::normalize_market_data_symbol;
use std::collections::{BTreeSet, HashMap, HashSet};

pub(super) const STANDARD_SYNC_TIMEFRAMES: [(&str, &str); 9] = [
    ("M1", "1Min"),
    ("M5", "5Min"),
    ("M15", "15Min"),
    ("M30", "30Min"),
    ("H1", "1Hour"),
    ("H4", "4Hour"),
    ("D1", "1Day"),
    ("W1", "1Week"),
    ("MN1", "1Month"),
];

const HIGH_TO_LOW_SYNC_TIMEFRAMES: [&str; 9] = [
    "1Month", "1Week", "1Day", "4Hour", "1Hour", "30Min", "15Min", "5Min", "1Min",
];

pub(super) const ALPACA_DEFAULT_HISTORICAL_RPM: u32 = 200;
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) enum AlpacaSyncBucket {
    Missing,
    Stale,
    Backfill,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct AlpacaSyncCandidate {
    pub symbol: String,
    pub timeframe: String,
    pub bucket: AlpacaSyncBucket,
    pub focus: bool,
    pub score: i64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct SyncCacheState {
    pub last_bar_ts_s: i64,
    pub write_ts_s: i64,
    pub bar_count: i64,
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

/// Persisted "bounded full-history fetch already exhausted available Alpaca
/// data" marker. Unlike `AlpacaNoDataPair`, these pairs still participate in
/// Missing/Stale sync; only repeat Backfill scheduling is suppressed.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub(super) struct AlpacaBackfillCompletePair {
    pub symbol: String,
    pub timeframe: String,
    #[serde(default)]
    pub marked_at: i64,
    #[serde(default)]
    pub bar_count: i64,
    #[serde(default)]
    pub target_bars: i64,
}

pub(super) fn normalize_sync_timeframe_key(tf: &str) -> Option<&'static str> {
    STANDARD_SYNC_TIMEFRAMES.iter().find_map(|(short, cache)| {
        if tf.eq_ignore_ascii_case(short) || tf.eq_ignore_ascii_case(cache) {
            Some(*cache)
        } else {
            None
        }
    })
}

pub(super) fn sync_timeframe_short_label(tf: &str) -> &str {
    STANDARD_SYNC_TIMEFRAMES
        .iter()
        .find_map(|(short, cache)| {
            if tf.eq_ignore_ascii_case(short) || tf.eq_ignore_ascii_case(cache) {
                Some(*short)
            } else {
                None
            }
        })
        .unwrap_or(tf)
}

pub(super) fn default_sync_timeframe_set() -> BTreeSet<String> {
    STANDARD_SYNC_TIMEFRAMES
        .iter()
        .map(|(_, cache)| (*cache).to_string())
        .collect()
}

pub(super) fn sync_timeframe_sort_key(tf: &str) -> usize {
    STANDARD_SYNC_TIMEFRAMES
        .iter()
        .position(|(_, cache)| tf.eq_ignore_ascii_case(cache))
        .unwrap_or(usize::MAX)
}

fn sync_timeframe_high_first_sort_key(tf: &str) -> usize {
    HIGH_TO_LOW_SYNC_TIMEFRAMES
        .iter()
        .position(|cache| tf.eq_ignore_ascii_case(cache))
        .unwrap_or(usize::MAX)
}

fn ordered_sync_timeframes_high_first(timeframes: &[String]) -> Vec<String> {
    let mut unique: Vec<String> = Vec::new();
    for timeframe in timeframes {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            continue;
        };
        if unique.iter().any(|existing| existing == tf) {
            continue;
        }
        unique.push(tf.to_string());
    }
    unique.sort_by_key(|tf| sync_timeframe_high_first_sort_key(tf));
    unique
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
            fetch_permits: 4,
            queue_window: 8,
            batch_size: 6,
            foreground_reserve: 3,
        },
        301..=1_500 => AlpacaSyncCapacity {
            fetch_permits: 6,
            queue_window: 12,
            batch_size: 8,
            foreground_reserve: 3,
        },
        1_501..=4_000 => AlpacaSyncCapacity {
            fetch_permits: 10,
            queue_window: 20,
            batch_size: 14,
            foreground_reserve: 4,
        },
        4_001..=7_000 => AlpacaSyncCapacity {
            fetch_permits: 12,
            queue_window: 24,
            batch_size: 18,
            foreground_reserve: 5,
        },
        _ => AlpacaSyncCapacity {
            fetch_permits: 16,
            queue_window: 32,
            batch_size: 24,
            foreground_reserve: 6,
        },
    }
}

pub(super) fn alpaca_fetch_key(symbol: &str, timeframe: &str) -> String {
    let sym = normalize_market_data_symbol(symbol).replace('/', "");
    let tf = normalize_sync_timeframe_key(timeframe).unwrap_or(timeframe);
    format!("{sym}:{tf}")
}

pub(super) fn alpaca_sync_target_bars(tf: &str) -> Option<u32> {
    match tf {
        "1Min" => Some(50_000),
        "5Min" => Some(50_000),
        "15Min" => Some(30_000),
        "30Min" => Some(30_000),
        "1Hour" => Some(30_000),
        // Keep high-TF targets inside Alpaca's bounded targeted lookback
        // windows so successful fetches can converge instead of thrashing.
        "4Hour" => Some(14_000),
        "1Day" => Some(3_500),
        "1Week" => Some(1_000),
        "1Month" => Some(240),
        _ => None,
    }
}

pub(super) fn kraken_sync_target_bars(tf: &str) -> Option<u32> {
    match tf {
        "1Min" | "5Min" | "15Min" | "30Min" | "1Hour" | "4Hour" | "1Day" | "1Week" => {
            Some(720)
        }
        "1Month" => Some(24),
        _ => None,
    }
}

pub(super) fn tastytrade_sync_target_bars(tf: &str) -> Option<u32> {
    match normalize_sync_timeframe_key(tf)? {
        // DXLink historical candle snapshots have materially shallower practical
        // limits than Alpaca. Keep targets below the provider cap so successful
        // fetches converge instead of repeatedly entering shallow-cache backfill.
        "1Min" | "5Min" | "15Min" | "30Min" | "1Hour" | "4Hour" => Some(7_500),
        "1Day" => Some(3_500),
        "1Week" => Some(1_000),
        "1Month" => Some(240),
        _ => None,
    }
}

fn classify_alpaca_sync_candidate(
    now_s: i64,
    symbol: &str,
    timeframe: &str,
    state: Option<SyncCacheState>,
    focus: bool,
    target_bars_for_tf: fn(&str) -> Option<u32>,
) -> Option<AlpacaSyncCandidate> {
    let timeframe = normalize_sync_timeframe_key(timeframe)?;
    let symbol = normalize_market_data_symbol(symbol).replace('/', "");
    let state = state.unwrap_or_default();
    let bar_count = state.bar_count;
    let age_anchor_s = if state.last_bar_ts_s > 0 {
        state.last_bar_ts_s
    } else {
        state.write_ts_s
    };
    if state.write_ts_s <= 0 || bar_count <= 0 {
        return Some(AlpacaSyncCandidate {
            symbol,
            timeframe: timeframe.to_string(),
            bucket: AlpacaSyncBucket::Missing,
            focus,
            score: 0,
        });
    }

    if let Some(period_s) = alpaca_sync_period_secs(timeframe) {
        let age_s = now_s.saturating_sub(age_anchor_s);
        if age_s >= period_s.saturating_mul(24) {
            return Some(AlpacaSyncCandidate {
                symbol,
                timeframe: timeframe.to_string(),
                bucket: AlpacaSyncBucket::Stale,
                focus,
                score: age_s,
            });
        }
    }

    if let Some(target_bars) = target_bars_for_tf(timeframe).map(i64::from)
        && bar_count * 100 < target_bars * 95
    {
        return Some(AlpacaSyncCandidate {
            symbol,
            timeframe: timeframe.to_string(),
            bucket: AlpacaSyncBucket::Backfill,
            focus,
            score: (target_bars - bar_count).max(0),
        });
    }

    None
}

pub(super) fn select_alpaca_sync_candidates(
    symbols: &[String],
    timeframes: &[String],
    state_map: &HashMap<(String, String), SyncCacheState>,
    focus_symbols: &HashSet<String>,
    no_data_pairs: &HashMap<String, AlpacaNoDataPair>,
    backfill_complete_pairs: &HashSet<String>,
    pending_fetches: &HashSet<String>,
    batch_size: usize,
    now_s: i64,
    target_bars_for_tf: fn(&str) -> Option<u32>,
) -> Vec<AlpacaSyncCandidate> {
    if batch_size == 0 || symbols.is_empty() || timeframes.is_empty() {
        return Vec::new();
    }

    let ordered_timeframes = ordered_sync_timeframes_high_first(timeframes);
    if ordered_timeframes.is_empty() {
        return Vec::new();
    }

    let mut missing_by_tf: HashMap<String, Vec<AlpacaSyncCandidate>> = HashMap::new();
    let mut stale_by_tf: HashMap<String, Vec<AlpacaSyncCandidate>> = HashMap::new();
    let mut backfill_by_tf: HashMap<String, Vec<AlpacaSyncCandidate>> = HashMap::new();

    for symbol in symbols {
        let symbol_key = normalize_market_data_symbol(symbol).replace('/', "");
        let focus = focus_symbols.contains(&symbol_key);
        for timeframe in timeframes {
            let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
                continue;
            };
            if no_data_pairs.contains_key(&alpaca_fetch_key(symbol, tf)) {
                continue;
            }
            let fetch_key = alpaca_fetch_key(symbol, tf);
            if pending_fetches.contains(&fetch_key) {
                continue;
            }
            let state = state_map.get(&(symbol_key.clone(), tf.to_string())).copied();
            let Some(candidate) =
                classify_alpaca_sync_candidate(
                    now_s,
                    &symbol_key,
                    tf,
                    state,
                    focus,
                    target_bars_for_tf,
                )
            else {
                continue;
            };
            if candidate.bucket == AlpacaSyncBucket::Backfill
                && backfill_complete_pairs.contains(&fetch_key)
            {
                continue;
            }
            match candidate.bucket {
                AlpacaSyncBucket::Missing => missing_by_tf
                    .entry(tf.to_string())
                    .or_default()
                    .push(candidate),
                AlpacaSyncBucket::Stale => stale_by_tf
                    .entry(tf.to_string())
                    .or_default()
                    .push(candidate),
                AlpacaSyncBucket::Backfill => backfill_by_tf
                    .entry(tf.to_string())
                    .or_default()
                    .push(candidate),
            }
        }
    }

    let selected_bucket_map = if missing_by_tf.values().any(|bucket| !bucket.is_empty()) {
        &mut missing_by_tf
    } else if stale_by_tf.values().any(|bucket| !bucket.is_empty()) {
        &mut stale_by_tf
    } else {
        &mut backfill_by_tf
    };

    let mut selected: Vec<AlpacaSyncCandidate> = Vec::with_capacity(batch_size);
    let sort_bucket = |bucket: &mut Vec<AlpacaSyncCandidate>| {
        bucket.sort_by(|a, b| {
            b.focus
                .cmp(&a.focus)
                .then(b.score.cmp(&a.score))
                .then(a.symbol.cmp(&b.symbol))
        });
    };

    for timeframe in &ordered_timeframes {
        let Some(bucket) = selected_bucket_map.get_mut(timeframe) else {
            continue;
        };
        sort_bucket(bucket);
        for candidate in bucket.drain(..) {
            selected.push(candidate);
            if selected.len() >= batch_size {
                return selected;
            }
        }
    }

    selected
}

pub(super) fn select_alpaca_sync_workset(
    symbols: &[String],
    timeframes: &[String],
    state_map: &HashMap<(String, String), SyncCacheState>,
    focus_symbols: &HashSet<String>,
    no_data_pairs: &HashMap<String, AlpacaNoDataPair>,
    backfill_complete_pairs: &HashSet<String>,
    pending_fetches: &HashSet<String>,
    batch_size: usize,
    foreground_slots: usize,
    now_s: i64,
    target_bars_for_tf: fn(&str) -> Option<u32>,
) -> Vec<AlpacaSyncCandidate> {
    if batch_size == 0 || timeframes.is_empty() {
        return Vec::new();
    }

    let mut selected: Vec<AlpacaSyncCandidate> = Vec::with_capacity(batch_size);
    let mut staged_pending = pending_fetches.clone();

    let mut foreground_symbols: Vec<String> = focus_symbols.iter().cloned().collect();
    foreground_symbols.sort();
    let foreground_budget = foreground_slots.min(batch_size);
    if foreground_budget > 0 && !foreground_symbols.is_empty() {
        let foreground = select_alpaca_sync_candidates(
            &foreground_symbols,
            timeframes,
            state_map,
            focus_symbols,
            no_data_pairs,
            backfill_complete_pairs,
            &staged_pending,
            foreground_budget,
            now_s,
            target_bars_for_tf,
        );
        for candidate in foreground {
            if staged_pending.insert(alpaca_fetch_key(&candidate.symbol, &candidate.timeframe)) {
                selected.push(candidate);
            }
        }
    }

    if selected.len() >= batch_size {
        return selected;
    }

    let background = select_alpaca_sync_candidates(
        symbols,
        timeframes,
        state_map,
        focus_symbols,
        no_data_pairs,
        backfill_complete_pairs,
        &staged_pending,
        batch_size - selected.len(),
        now_s,
        target_bars_for_tf,
    );
    for candidate in background {
        if selected.len() >= batch_size {
            break;
        }
        if staged_pending.insert(alpaca_fetch_key(&candidate.symbol, &candidate.timeframe)) {
            selected.push(candidate);
        }
    }

    selected
}

pub(super) fn sync_timeframe_period_secs(timeframe: &str) -> Option<i64> {
    match normalize_sync_timeframe_key(timeframe)? {
        "1Min" => Some(60),
        "5Min" => Some(300),
        "15Min" => Some(900),
        "30Min" => Some(1800),
        "1Hour" => Some(3600),
        "4Hour" => Some(14400),
        "1Day" => Some(86400),
        "1Week" => Some(604800),
        "1Month" => Some(2592000),
        _ => None,
    }
}

fn alpaca_sync_period_secs(timeframe: &str) -> Option<i64> {
    sync_timeframe_period_secs(timeframe)
}

pub(super) fn alpaca_incremental_fetch_limit_at(
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
    fn normalize_sync_timeframe_key_accepts_short_and_cache_labels() {
        assert_eq!(normalize_sync_timeframe_key("M1"), Some("1Min"));
        assert_eq!(normalize_sync_timeframe_key("1Min"), Some("1Min"));
        assert_eq!(normalize_sync_timeframe_key("mn1"), Some("1Month"));
        assert_eq!(normalize_sync_timeframe_key("1Month"), Some("1Month"));
        assert_eq!(normalize_sync_timeframe_key("bogus"), None);
    }

    #[test]
    fn sync_timeframe_short_label_maps_cache_suffixes() {
        assert_eq!(sync_timeframe_short_label("1Min"), "M1");
        assert_eq!(sync_timeframe_short_label("1Hour"), "H1");
        assert_eq!(sync_timeframe_short_label("1Month"), "MN1");
    }

    #[test]
    fn ordered_sync_timeframes_high_first_dedupes_and_normalizes() {
        let ordered = ordered_sync_timeframes_high_first(&[
            "M1".to_string(),
            "1Day".to_string(),
            "MN1".to_string(),
            "1Min".to_string(),
            "1Day".to_string(),
            "bogus".to_string(),
        ]);
        assert_eq!(
            ordered,
            vec![
                "1Month".to_string(),
                "1Day".to_string(),
                "1Min".to_string(),
            ]
        );
    }

    #[test]
    fn select_alpaca_sync_candidates_prioritizes_missing_before_stale_or_backfill() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "TSLA".to_string()];
        let timeframes = vec!["1Day".to_string()];
        let mut state_map = HashMap::new();
        state_map.insert(
            ("MSFT".to_string(), "1Day".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 25 * 86_400,
                write_ts_s: now_s - 7 * 3600,
                bar_count: 10_000,
            },
        );
        state_map.insert(
            ("TSLA".to_string(), "1Day".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 60,
                write_ts_s: now_s - 60,
                bar_count: 100,
            },
        );

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &state_map,
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            3,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].symbol, "AAPL");
        assert_eq!(selected[0].bucket, AlpacaSyncBucket::Missing);
    }

    #[test]
    fn select_alpaca_sync_candidates_prefers_focus_symbols_within_bucket() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string(), "MSFT".to_string()];
        let timeframes = vec!["1Day".to_string()];
        let focus = HashSet::from(["MSFT".to_string()]);

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &focus,
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            2,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].symbol, "MSFT");
        assert_eq!(selected[1].symbol, "AAPL");
        assert!(selected.iter().all(|c| c.bucket == AlpacaSyncBucket::Missing));
    }

    #[test]
    fn select_alpaca_sync_candidates_skips_known_no_data_pairs() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAGIY".to_string(), "AAPL".to_string()];
        let timeframes = vec!["1Hour".to_string()];
        let no_data = HashMap::from([(
            alpaca_fetch_key("AAGIY", "1Hour"),
            AlpacaNoDataPair {
                symbol: "AAGIY".to_string(),
                timeframe: "1Hour".to_string(),
                marked_at: now_s,
                reason: "No bar data for AAGIY @ 1Hour".to_string(),
            },
        )]);

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &HashSet::new(),
            &no_data,
            &HashSet::new(),
            &HashSet::new(),
            2,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].symbol, "AAPL");
        assert_eq!(selected[0].timeframe, "1Hour");
    }

    #[test]
    fn select_alpaca_sync_candidates_prioritizes_higher_timeframes_across_symbols() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string(), "MSFT".to_string()];
        let timeframes = vec!["1Min".to_string(), "1Day".to_string(), "1Month".to_string()];

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            4,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 4);
        assert!(selected.iter().all(|c| c.bucket == AlpacaSyncBucket::Missing));
        assert_eq!(selected[0].timeframe, "1Month");
        assert_eq!(selected[1].timeframe, "1Month");
        assert_eq!(selected[2].timeframe, "1Day");
        assert_eq!(selected[3].timeframe, "1Day");
    }

    #[test]
    fn select_alpaca_sync_workset_reserves_slots_for_focus_symbols() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string(), "MSFT".to_string()];
        let timeframes = vec!["1Day".to_string()];
        let focus = HashSet::from(["MSFT".to_string()]);
        let state_map = HashMap::from([(
            ("MSFT".to_string(), "1Day".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 25 * 86_400,
                write_ts_s: now_s - 7 * 3600,
                bar_count: 10_000,
            },
        )]);

        let selected = select_alpaca_sync_workset(
            &symbols,
            &timeframes,
            &state_map,
            &focus,
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            2,
            1,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].symbol, "MSFT");
        assert_eq!(selected[0].bucket, AlpacaSyncBucket::Stale);
        assert_eq!(selected[1].symbol, "AAPL");
        assert_eq!(selected[1].bucket, AlpacaSyncBucket::Missing);
    }

    #[test]
    fn select_alpaca_sync_workset_dedupes_focus_candidates_from_background() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["MSFT".to_string()];
        let timeframes = vec!["1Day".to_string(), "1Hour".to_string()];
        let focus = HashSet::from(["MSFT".to_string()]);

        let selected = select_alpaca_sync_workset(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &focus,
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            2,
            1,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].symbol, "MSFT");
        assert_eq!(selected[1].symbol, "MSFT");
        assert_ne!(selected[0].timeframe, selected[1].timeframe);
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
    fn stale_window_uses_timeframe_last_bar_age_not_write_age() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string()];
        let timeframes = vec!["1Min".to_string()];
        let state_map = HashMap::from([(
            ("AAPL".to_string(), "1Min".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 25 * 60,
                write_ts_s: now_s - 60,
                bar_count: 50_000,
            },
        )]);

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &state_map,
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            1,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].bucket, AlpacaSyncBucket::Stale);
    }

    #[test]
    fn kraken_limited_history_target_does_not_force_permanent_backfill() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["BTCUSD".to_string()];
        let timeframes = vec!["1Min".to_string()];
        let state_map = HashMap::from([(
            ("BTCUSD".to_string(), "1Min".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 60,
                write_ts_s: now_s - 60,
                bar_count: 720,
            },
        )]);

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &state_map,
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            1,
            now_s,
            kraken_sync_target_bars,
        );

        assert!(selected.is_empty());
    }

    #[test]
    fn tastytrade_targets_fit_dxlink_snapshot_depth() {
        assert_eq!(tastytrade_sync_target_bars("1Min"), Some(7_500));
        assert_eq!(tastytrade_sync_target_bars("4Hour"), Some(7_500));
        assert_eq!(tastytrade_sync_target_bars("1Day"), Some(3_500));
        assert_eq!(tastytrade_sync_target_bars("1Month"), Some(240));
    }

    #[test]
    fn tastytrade_limited_history_target_does_not_force_permanent_backfill() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string()];
        let timeframes = vec!["1Hour".to_string()];
        let state_map = HashMap::from([(
            ("AAPL".to_string(), "1Hour".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 3600,
                write_ts_s: now_s - 60,
                bar_count: 7_500,
            },
        )]);

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &state_map,
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            1,
            now_s,
            tastytrade_sync_target_bars,
        );

        assert!(selected.is_empty());
    }

    #[test]
    fn select_alpaca_sync_candidates_skips_backfill_complete_pairs() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["LUMN".to_string()];
        let timeframes = vec!["1Month".to_string()];
        let state_map = HashMap::from([(
            ("LUMN".to_string(), "1Month".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 60,
                write_ts_s: now_s - 60,
                bar_count: 70,
            },
        )]);
        let backfill_complete = HashSet::from([alpaca_fetch_key("LUMN", "1Month")]);

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &state_map,
            &HashSet::new(),
            &HashMap::new(),
            &backfill_complete,
            &HashSet::new(),
            1,
            now_s,
            alpaca_sync_target_bars,
        );

        assert!(selected.is_empty());
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
