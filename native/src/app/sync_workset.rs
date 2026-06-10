//! Broker-agnostic sync scheduler primitives.
//!
//! Houses the shared bar-sync workset selection logic that every broker
//! integration plugs into: timeframe normalization, the candidate bucket
//! (Missing / Stale / Backfill), focus-vs-background ordering, and the
//! rotating high-TF-first ring used by Alpaca, Kraken, and Tastytrade
//! scheduling alike. Broker-specific knobs (target_bars, RPM presets,
//! no-data persistence) live next door in the per-broker sync modules.
//!
//! Existing names with an `Alpaca` prefix (`AlpacaSyncCandidate`,
//! `AlpacaSyncBucket`, `AlpacaBackfillCompletePair`, `alpaca_fetch_key`,
//! `select_alpaca_sync_*`) are retained as-is for now: they are misleading
//! but stable across ~170 internal call sites. Renaming is deferred.

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

/// Persisted "bounded full-history fetch already exhausted available data"
/// marker. Pairs still participate in Missing/Stale sync; only repeat
/// Backfill scheduling is suppressed.
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
    let mut seen: HashSet<&'static str> =
        HashSet::with_capacity(timeframes.len().min(STANDARD_SYNC_TIMEFRAMES.len()));
    let mut unique: Vec<String> =
        Vec::with_capacity(timeframes.len().min(STANDARD_SYNC_TIMEFRAMES.len()));
    for timeframe in timeframes {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            continue;
        };
        if !seen.insert(tf) {
            continue;
        }
        unique.push(tf.to_string());
    }
    unique.sort_by_key(|tf| sync_timeframe_high_first_sort_key(tf));
    unique
}

pub(super) fn alpaca_fetch_key(symbol: &str, timeframe: &str) -> String {
    let sym = normalize_market_data_symbol(symbol).replace('/', "");
    let tf = normalize_sync_timeframe_key(timeframe).unwrap_or(timeframe);
    format!("{sym}:{tf}")
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

pub(super) fn alpaca_sync_period_secs(timeframe: &str) -> Option<i64> {
    sync_timeframe_period_secs(timeframe)
}

pub(super) fn merge_recent_sync_overrides(
    rebuilt: &mut HashMap<(String, String), SyncCacheState>,
    previous: &HashMap<(String, String), SyncCacheState>,
    now_s: i64,
) {
    for (key, prior) in previous {
        if prior.write_ts_s <= 0 || prior.bar_count <= 0 {
            continue;
        }
        let Some(period_s) = sync_timeframe_period_secs(&key.1) else {
            continue;
        };
        if now_s.saturating_sub(prior.write_ts_s) > period_s.saturating_mul(24) {
            continue;
        }
        let replace = rebuilt
            .get(key)
            .map(|current| prior.write_ts_s > current.write_ts_s)
            .unwrap_or(true);
        if replace {
            rebuilt.insert(key.clone(), *prior);
        }
    }
}

fn foreground_sync_write_cooldown_secs(period_s: i64) -> i64 {
    (period_s / 2).clamp(30, 300)
}

pub(super) fn classify_alpaca_sync_candidate(
    now_s: i64,
    symbol: &str,
    timeframe: &str,
    state: Option<SyncCacheState>,
    focus: bool,
    target_bars_for_tf: fn(&str) -> Option<u32>,
) -> Option<AlpacaSyncCandidate> {
    classify_alpaca_sync_candidate_with_stale_multiplier(
        now_s,
        symbol,
        timeframe,
        state,
        focus,
        24,
        target_bars_for_tf,
    )
}

pub(super) fn classify_alpaca_sync_candidate_with_stale_multiplier(
    now_s: i64,
    symbol: &str,
    timeframe: &str,
    state: Option<SyncCacheState>,
    focus: bool,
    background_stale_periods: i64,
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
        let write_age_s = now_s.saturating_sub(state.write_ts_s.max(0));
        // Foreground charts (including every MTF_Grid cell) are trading-session
        // critical. Do not let the broad-universe 24×TF stale window decide
        // whether they refresh: once the current timeframe has elapsed, they
        // are due, but still respect a per-TF write cooldown so a provider that
        // has not printed the next bar yet cannot be hammered every scheduler tick.
        let stale_due = if focus {
            age_s >= period_s && write_age_s >= foreground_sync_write_cooldown_secs(period_s)
        } else {
            let stale_periods = background_stale_periods.max(1);
            age_s >= period_s.saturating_mul(stale_periods)
        };
        if stale_due {
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

#[cfg(test)]
fn select_alpaca_sync_candidates_from_iter<'a, I>(
    symbols: I,
    timeframes: &[String],
    state_map: &HashMap<(String, String), SyncCacheState>,
    focus_symbols: &HashSet<String>,
    no_data_keys: &HashSet<String>,
    backfill_complete_pairs: &HashMap<String, AlpacaBackfillCompletePair>,
    pending_fetches: &HashSet<String>,
    batch_size: usize,
    now_s: i64,
    target_bars_for_tf: fn(&str) -> Option<u32>,
) -> Vec<AlpacaSyncCandidate>
where
    I: IntoIterator<Item = &'a str>,
{
    if batch_size == 0 || timeframes.is_empty() {
        return Vec::new();
    }

    // Protect foreground (MTF Grid / focus) work from being starved by background
    let has_foreground = !focus_symbols.is_empty();
    let effective_batch = adjust_batch_for_tier_protection(batch_size, has_foreground);

    let ordered_timeframes = ordered_sync_timeframes_high_first(timeframes);
    if ordered_timeframes.is_empty() {
        return Vec::new();
    }

    let bucket_capacity = batch_size.saturating_mul(2).max(ordered_timeframes.len());
    let mut missing_by_tf: HashMap<&'static str, Vec<AlpacaSyncCandidate>> =
        HashMap::with_capacity(ordered_timeframes.len());
    let mut stale_by_tf: HashMap<&'static str, Vec<AlpacaSyncCandidate>> =
        HashMap::with_capacity(ordered_timeframes.len());
    let mut backfill_by_tf: HashMap<&'static str, Vec<AlpacaSyncCandidate>> =
        HashMap::with_capacity(ordered_timeframes.len());

    for symbol in symbols {
        let symbol_key = normalize_market_data_symbol(symbol).replace('/', "");
        if symbol_key.is_empty() {
            continue;
        }
        let focus = focus_symbols.contains(&symbol_key);
        for timeframe in timeframes {
            let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
                continue;
            };
            let fetch_key = alpaca_fetch_key(&symbol_key, tf);
            if no_data_keys.contains(&fetch_key) || pending_fetches.contains(&fetch_key) {
                continue;
            }
            let state = state_map
                .get(&(symbol_key.clone(), tf.to_string()))
                .copied();
            let Some(candidate) = classify_alpaca_sync_candidate(
                now_s,
                &symbol_key,
                tf,
                state,
                focus,
                target_bars_for_tf,
            ) else {
                continue;
            };
            if candidate.bucket == AlpacaSyncBucket::Backfill
                && backfill_complete_pairs.contains_key(&fetch_key)
            {
                continue;
            }
            match candidate.bucket {
                AlpacaSyncBucket::Missing => missing_by_tf
                    .entry(tf)
                    .or_insert_with(|| Vec::with_capacity(bucket_capacity))
                    .push(candidate),
                AlpacaSyncBucket::Stale => stale_by_tf
                    .entry(tf)
                    .or_insert_with(|| Vec::with_capacity(bucket_capacity))
                    .push(candidate),
                AlpacaSyncBucket::Backfill => backfill_by_tf
                    .entry(tf)
                    .or_insert_with(|| Vec::with_capacity(bucket_capacity))
                    .push(candidate),
            }
        }
    }

    let mut selected: Vec<AlpacaSyncCandidate> = Vec::with_capacity(batch_size);

    drain_buckets_high_tf_first(
        &mut selected,
        &mut missing_by_tf,
        &ordered_timeframes,
        batch_size,
    );
    if !selected.is_empty() {
        return selected;
    }
    drain_buckets_high_tf_first(
        &mut selected,
        &mut stale_by_tf,
        &ordered_timeframes,
        batch_size,
    );
    drain_buckets_high_tf_first(
        &mut selected,
        &mut missing_by_tf,
        &ordered_timeframes,
        batch_size,
    );
    drain_buckets_high_tf_first(
        &mut selected,
        &mut backfill_by_tf,
        &ordered_timeframes,
        batch_size,
    );

    selected
}

/// Drain `buckets` into `selected` in the supplied timeframe order, stopping
/// when `selected.len() >= cap`. Within a single timeframe bucket, sorts by
/// (focus desc, score desc, symbol asc) so foreground charts win and the most-
/// stale entries fetch first.
#[cfg(test)]
fn drain_buckets_high_tf_first(
    selected: &mut Vec<AlpacaSyncCandidate>,
    buckets: &mut HashMap<&'static str, Vec<AlpacaSyncCandidate>>,
    ordered_timeframes: &[String],
    cap: usize,
) {
    if selected.len() >= cap {
        return;
    }
    for timeframe in ordered_timeframes {
        if selected.len() >= cap {
            return;
        }
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            continue;
        };
        let Some(bucket) = buckets.get_mut(tf) else {
            continue;
        };
        if bucket.is_empty() {
            continue;
        }
        bucket.sort_by(|a, b| {
            b.focus
                .cmp(&a.focus)
                .then(b.score.cmp(&a.score))
                .then(a.symbol.cmp(&b.symbol))
        });
        let want = cap.saturating_sub(selected.len());
        let take = want.min(bucket.len());
        selected.extend(bucket.drain(..take));
    }
}

#[cfg(test)]
pub(super) fn select_alpaca_sync_candidates(
    symbols: &[String],
    timeframes: &[String],
    state_map: &HashMap<(String, String), SyncCacheState>,
    focus_symbols: &HashSet<String>,
    no_data_keys: &HashSet<String>,
    backfill_complete_pairs: &HashMap<String, AlpacaBackfillCompletePair>,
    pending_fetches: &HashSet<String>,
    batch_size: usize,
    now_s: i64,
    target_bars_for_tf: fn(&str) -> Option<u32>,
) -> Vec<AlpacaSyncCandidate> {
    if symbols.is_empty() {
        return Vec::new();
    }
    select_alpaca_sync_candidates_from_iter(
        symbols.iter().map(String::as_str),
        timeframes,
        state_map,
        focus_symbols,
        no_data_keys,
        backfill_complete_pairs,
        pending_fetches,
        batch_size,
        now_s,
        target_bars_for_tf,
    )
}

#[cfg(test)]
pub(super) fn select_alpaca_sync_workset(
    symbols: &[String],
    timeframes: &[String],
    state_map: &HashMap<(String, String), SyncCacheState>,
    focus_symbols: &HashSet<String>,
    no_data_keys: &HashSet<String>,
    backfill_complete_pairs: &HashMap<String, AlpacaBackfillCompletePair>,
    pending_fetches: &HashSet<String>,
    batch_size: usize,
    foreground_slots: usize,
    now_s: i64,
    target_bars_for_tf: fn(&str) -> Option<u32>,
) -> Vec<AlpacaSyncCandidate> {
    if batch_size == 0 || timeframes.is_empty() {
        return Vec::new();
    }

    // Protect foreground (MTF Grid / focus) work from being starved by background
    let has_foreground = !focus_symbols.is_empty();
    let effective_batch = adjust_batch_for_tier_protection(batch_size, has_foreground);

    let mut selected: Vec<AlpacaSyncCandidate> = Vec::with_capacity(batch_size);
    let mut staged_pending = pending_fetches.clone();

    // Apply bounded concurrency protection for foreground work
    let has_foreground = !focus_symbols.is_empty();
    let effective_batch = adjust_batch_for_tier_protection(batch_size, has_foreground);

    // === Tiered Priority + High-Timeframe-First ===
    // 1. MTF Grid / focused symbols get highest priority
    // 2. Within each tier, timeframes are processed high-to-low
    // 3. Coverage-first (Missing) still respects tiers

    // Coverage-first mode: if any candidate has no cached bars at all, fill
    // those gaps highest timeframe -> lowest before spending slots on stale
    // refreshes or provider-history backfill. Focus still sorts within a bucket,
    // but it must not allow active-chart refreshes to starve initial coverage.
    let coverage = select_alpaca_sync_candidates(
        symbols,
        timeframes,
        state_map,
        focus_symbols,
        no_data_keys,
        backfill_complete_pairs,
        &staged_pending,
        batch_size,
        now_s,
        target_bars_for_tf,
    );
    if coverage
        .first()
        .is_some_and(|candidate| candidate.bucket == AlpacaSyncBucket::Missing)
    {
        // Still apply tier + TF ordering even in coverage-first mode
        let mtf_grid: HashSet<String> = focus_symbols.iter().cloned().collect();
        let ordered_coverage = sort_candidates_by_priority_then_timeframe(
            coverage,
            &mtf_grid,
            focus_symbols,
            &HashSet::new(),
            &HashSet::new(),
        );
        for candidate in ordered_coverage {
            if staged_pending.insert(alpaca_fetch_key(&candidate.symbol, &candidate.timeframe)) {
                selected.push(candidate);
                if selected.len() >= batch_size {
                    break;
                }
            }
        }
        return selected;
    }

    let mut foreground_symbols: Vec<String> = focus_symbols.iter().cloned().collect();
    foreground_symbols.sort();
    let foreground_budget = foreground_slots.min(batch_size);
    if foreground_budget > 0 && !foreground_symbols.is_empty() {
        let foreground = select_alpaca_sync_candidates(
            &foreground_symbols,
            timeframes,
            state_map,
            focus_symbols,
            no_data_keys,
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
        no_data_keys,
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

pub(super) fn select_alpaca_sync_workset_rotating(
    symbols: &[String],
    timeframes: &[String],
    state_map: &HashMap<(String, String), SyncCacheState>,
    focus_symbols: &HashSet<String>,
    no_data_keys: &HashSet<String>,
    backfill_complete_pairs: &HashMap<String, AlpacaBackfillCompletePair>,
    pending_fetches: &HashSet<String>,
    batch_size: usize,
    foreground_slots: usize,
    background_scan_limit: usize,
    cursor: &mut usize,
    now_s: i64,
    target_bars_for_tf: fn(&str) -> Option<u32>,
) -> Vec<AlpacaSyncCandidate> {
    select_alpaca_sync_workset_rotating_with_stale_multiplier(
        symbols,
        timeframes,
        state_map,
        focus_symbols,
        no_data_keys,
        backfill_complete_pairs,
        pending_fetches,
        batch_size,
        foreground_slots,
        background_scan_limit,
        cursor,
        now_s,
        24,
        target_bars_for_tf,
    )
}

pub(super) fn select_alpaca_sync_workset_rotating_with_stale_multiplier(
    symbols: &[String],
    timeframes: &[String],
    state_map: &HashMap<(String, String), SyncCacheState>,
    focus_symbols: &HashSet<String>,
    no_data_keys: &HashSet<String>,
    backfill_complete_pairs: &HashMap<String, AlpacaBackfillCompletePair>,
    pending_fetches: &HashSet<String>,
    batch_size: usize,
    _foreground_slots: usize,
    background_scan_limit: usize,
    cursor: &mut usize,
    now_s: i64,
    background_stale_periods: i64,
    target_bars_for_tf: fn(&str) -> Option<u32>,
) -> Vec<AlpacaSyncCandidate> {
    if batch_size == 0 || symbols.is_empty() || timeframes.is_empty() {
        return Vec::new();
    }

    let ordered_timeframes = ordered_sync_timeframes_high_first(timeframes);
    if ordered_timeframes.is_empty() {
        return Vec::new();
    }

    let total_symbols = symbols.len();
    let background_scan_limit = background_scan_limit.max(batch_size).min(total_symbols);
    let symbol_start = *cursor % total_symbols;

    // Strict high-timeframe-first mode: each refill chooses the highest
    // timeframe that still has actionable work, then spends the whole batch on
    // that timeframe. This matches the product goal for merged bars:
    // MN1/all symbols -> W1/all symbols -> D1 -> H4 -> H1 -> lower TFs.
    // Focus/open symbols sort first *within* the chosen timeframe, but they do
    // not let a lower-timeframe chart refresh preempt incomplete higher-TF
    // coverage/backfill for the merged dataset.
    for timeframe in &ordered_timeframes {
        let Some(tf) = normalize_sync_timeframe_key(timeframe) else {
            continue;
        };

        let mut staged_selected = pending_fetches.clone();
        let mut missing: Vec<AlpacaSyncCandidate> = Vec::with_capacity(batch_size);
        let mut stale: Vec<AlpacaSyncCandidate> = Vec::with_capacity(batch_size);
        let mut backfill: Vec<AlpacaSyncCandidate> = Vec::with_capacity(batch_size);

        // Always test focused/position/watchlist symbols for the current high
        // timeframe before rotating background symbols. This preserves trading
        // session usefulness without breaking high-TF-first ordering.
        if !matches!(tf, "1Min" | "5Min") {
            let mut foreground_symbols: Vec<&str> =
                focus_symbols.iter().map(String::as_str).collect();
            foreground_symbols.sort_unstable();
            for symbol in foreground_symbols {
                collect_sync_candidate_for_timeframe(
                    symbol,
                    tf,
                    state_map,
                    focus_symbols,
                    no_data_keys,
                    backfill_complete_pairs,
                    &mut staged_selected,
                    now_s,
                    background_stale_periods,
                    target_bars_for_tf,
                    &mut missing,
                    &mut stale,
                    &mut backfill,
                );
            }
        }

        let mut scanned = 0usize;
        while scanned < total_symbols {
            let scan_window = background_scan_limit.min(total_symbols - scanned);
            for offset in 0..scan_window {
                let symbol_idx = (symbol_start + scanned + offset) % total_symbols;
                collect_sync_candidate_for_timeframe(
                    &symbols[symbol_idx],
                    tf,
                    state_map,
                    focus_symbols,
                    no_data_keys,
                    backfill_complete_pairs,
                    &mut staged_selected,
                    now_s,
                    background_stale_periods,
                    target_bars_for_tf,
                    &mut missing,
                    &mut stale,
                    &mut backfill,
                );
            }

            if !(missing.is_empty() && stale.is_empty() && backfill.is_empty()) {
                *cursor = (symbol_start + scanned + scan_window) % total_symbols;

                let mut selected: Vec<AlpacaSyncCandidate> = Vec::with_capacity(batch_size);
                sort_sync_bucket(&mut missing);
                sort_sync_bucket(&mut stale);
                sort_sync_bucket(&mut backfill);

                take_sync_bucket(&mut selected, &mut missing, batch_size);
                take_sync_bucket(&mut selected, &mut stale, batch_size);
                take_sync_bucket(&mut selected, &mut backfill, batch_size);
                return selected;
            }

            scanned = scanned.saturating_add(background_scan_limit);
        }
    }

    *cursor = (symbol_start + background_scan_limit) % total_symbols;
    Vec::new()
}

#[allow(clippy::too_many_arguments)]
fn collect_sync_candidate_for_timeframe(
    symbol: &str,
    tf: &'static str,
    state_map: &HashMap<(String, String), SyncCacheState>,
    focus_symbols: &HashSet<String>,
    no_data_keys: &HashSet<String>,
    backfill_complete_pairs: &HashMap<String, AlpacaBackfillCompletePair>,
    staged_selected: &mut HashSet<String>,
    now_s: i64,
    background_stale_periods: i64,
    target_bars_for_tf: fn(&str) -> Option<u32>,
    missing: &mut Vec<AlpacaSyncCandidate>,
    stale: &mut Vec<AlpacaSyncCandidate>,
    backfill: &mut Vec<AlpacaSyncCandidate>,
) {
    let symbol_key = normalize_market_data_symbol(symbol).replace('/', "");
    if symbol_key.is_empty() {
        return;
    }
    let fetch_key = alpaca_fetch_key(&symbol_key, tf);
    if no_data_keys.contains(&fetch_key) || !staged_selected.insert(fetch_key.clone()) {
        return;
    }
    let focus = focus_symbols.contains(&symbol_key);
    let state = state_map
        .get(&(symbol_key.clone(), tf.to_string()))
        .copied();
    let Some(candidate) = classify_alpaca_sync_candidate_with_stale_multiplier(
        now_s,
        &symbol_key,
        tf,
        state,
        focus,
        background_stale_periods,
        target_bars_for_tf,
    ) else {
        return;
    };
    if candidate.bucket == AlpacaSyncBucket::Backfill
        && backfill_complete_pairs.contains_key(&fetch_key)
    {
        return;
    }
    match candidate.bucket {
        AlpacaSyncBucket::Missing => missing.push(candidate),
        AlpacaSyncBucket::Stale => stale.push(candidate),
        AlpacaSyncBucket::Backfill => backfill.push(candidate),
    }
}

fn sort_sync_bucket(bucket: &mut [AlpacaSyncCandidate]) {
    bucket.sort_by(|a, b| {
        b.focus
            .cmp(&a.focus)
            .then(b.score.cmp(&a.score))
            .then(a.symbol.cmp(&b.symbol))
    });
}

fn take_sync_bucket(
    selected: &mut Vec<AlpacaSyncCandidate>,
    bucket: &mut Vec<AlpacaSyncCandidate>,
    cap: usize,
) {
    let want = cap.saturating_sub(selected.len());
    let take = want.min(bucket.len());
    selected.extend(bucket.drain(..take));
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpaca_sync_target_bars(tf: &str) -> Option<u32> {
        normalize_sync_timeframe_key(tf).map(|_| u32::MAX)
    }

    fn provider_window_target_bars(tf: &str) -> Option<u32> {
        normalize_sync_timeframe_key(tf)?;
        None
    }

    #[test]
    fn provider_window_native_lane_can_refresh_after_one_period() {
        let now_s = 1_700_000_000i64;
        let state = Some(SyncCacheState {
            last_bar_ts_s: now_s - 2 * 86_400,
            write_ts_s: now_s - 3600,
            bar_count: 40,
        });

        assert!(
            classify_alpaca_sync_candidate(
                now_s,
                "TNDM",
                "1Day",
                state,
                false,
                provider_window_target_bars,
            )
            .is_none()
        );

        let candidate = classify_alpaca_sync_candidate_with_stale_multiplier(
            now_s,
            "TNDM",
            "1Day",
            state,
            false,
            1,
            provider_window_target_bars,
        )
        .expect("one-period native provider-window refresh should be due");
        assert_eq!(candidate.bucket, AlpacaSyncBucket::Stale);
    }

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
            vec!["1Month".to_string(), "1Day".to_string(), "1Min".to_string(),]
        );
    }

    #[test]
    fn sync_timeframe_period_secs_covers_standard_buckets() {
        assert_eq!(sync_timeframe_period_secs("M1"), Some(60));
        assert_eq!(sync_timeframe_period_secs("1Hour"), Some(3600));
        assert_eq!(sync_timeframe_period_secs("MN1"), Some(2_592_000));
        assert!(sync_timeframe_period_secs("bogus").is_none());
    }

    #[test]
    fn alpaca_fetch_key_normalizes_and_strips_slashes() {
        assert_eq!(alpaca_fetch_key("BTC/USD", "H4"), "BTCUSD:4Hour");
        assert_eq!(alpaca_fetch_key("aapl", "1Day"), "AAPL:1Day");
    }

    #[test]
    fn default_sync_timeframe_set_lists_all_nine_buckets() {
        let set = default_sync_timeframe_set();
        assert_eq!(set.len(), 9);
        assert!(set.contains("1Min"));
        assert!(set.contains("1Month"));
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
            &HashSet::new(),
            &HashMap::new(),
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
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            2,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].symbol, "MSFT");
        assert_eq!(selected[1].symbol, "AAPL");
        assert!(
            selected
                .iter()
                .all(|c| c.bucket == AlpacaSyncBucket::Missing)
        );
    }

    #[test]
    fn select_alpaca_sync_candidates_skips_known_no_data_pairs() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAGIY".to_string(), "AAPL".to_string()];
        let timeframes = vec!["1Hour".to_string()];
        let no_data_keys = HashSet::from([alpaca_fetch_key("AAGIY", "1Hour")]);

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &HashSet::new(),
            &no_data_keys,
            &HashMap::new(),
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
    fn select_alpaca_sync_candidates_uses_normalized_pending_keys() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["BTC/USD".to_string(), "ETH/USD".to_string()];
        let timeframes = vec!["H4".to_string()];
        let pending = HashSet::from([alpaca_fetch_key("BTCUSD", "4Hour")]);

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            &HashMap::new(),
            &pending,
            2,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].symbol, "ETHUSD");
        assert_eq!(selected[0].timeframe, "4Hour");
    }

    #[test]
    fn merge_recent_sync_overrides_preserves_settled_fetch_across_bg_rev_rebuild() {
        let now_s = 1_700_000_000i64;
        let mut rebuilt = HashMap::from([(
            ("GDC".to_string(), "4Hour".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 400 * 14_400,
                write_ts_s: now_s - 400 * 14_400,
                bar_count: 1_422,
            },
        )]);
        let previous = HashMap::from([(
            ("GDC".to_string(), "4Hour".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 30,
                write_ts_s: now_s - 30,
                bar_count: 1_422,
            },
        )]);

        merge_recent_sync_overrides(&mut rebuilt, &previous, now_s);

        let state = rebuilt
            .get(&("GDC".to_string(), "4Hour".to_string()))
            .expect("recent override should remain indexed");
        assert_eq!(state.write_ts_s, now_s - 30);
        assert_eq!(state.last_bar_ts_s, now_s - 30);
    }

    #[test]
    fn select_alpaca_sync_candidates_backfill_marker_does_not_hide_stale_pair() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["GDC".to_string()];
        let timeframes = vec!["4Hour".to_string()];
        let state_map = HashMap::from([(
            ("GDC".to_string(), "4Hour".to_string()),
            SyncCacheState {
                last_bar_ts_s: now_s - 25 * 14_400,
                write_ts_s: now_s - 60,
                bar_count: 1_422,
            },
        )]);
        let backfill_complete = HashMap::from([(
            alpaca_fetch_key("GDC", "4Hour"),
            AlpacaBackfillCompletePair {
                symbol: "GDC".to_string(),
                timeframe: "4Hour".to_string(),
                marked_at: now_s - 30,
                bar_count: 1_422,
                target_bars: 14_000,
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
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].symbol, "GDC");
        assert_eq!(selected[0].bucket, AlpacaSyncBucket::Stale);
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
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            4,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 4);
        assert!(
            selected
                .iter()
                .all(|c| c.bucket == AlpacaSyncBucket::Missing)
        );
        assert_eq!(selected[0].timeframe, "1Month");
        assert_eq!(selected[1].timeframe, "1Month");
        assert_eq!(selected[2].timeframe, "1Day");
        assert_eq!(selected[3].timeframe, "1Day");
    }

    #[test]
    fn select_alpaca_sync_workset_prioritizes_missing_before_focus_refresh() {
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
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            2,
            1,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].symbol, "AAPL");
        assert_eq!(selected[0].bucket, AlpacaSyncBucket::Missing);
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
            &HashSet::new(),
            &HashMap::new(),
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
    fn select_alpaca_sync_workset_rotating_bounds_background_scan() {
        let now_s = 1_700_000_000i64;
        let symbols = vec![
            "AAPL".to_string(),
            "MSFT".to_string(),
            "QQQ".to_string(),
            "TSLA".to_string(),
        ];
        let timeframes = vec!["1Day".to_string()];
        let mut cursor = 0usize;

        let first = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            1,
            0,
            2,
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(first.len(), 1);
        assert_eq!(first[0].symbol, "AAPL");
        assert_eq!(cursor, 2);

        let second = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            1,
            0,
            2,
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(second.len(), 1);
        assert_eq!(second[0].symbol, "QQQ");
        assert_eq!(cursor, 0);
    }

    #[test]
    fn select_alpaca_sync_workset_rotating_prioritizes_focus_before_background_scan() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "QQQ".to_string()];
        let timeframes = vec!["1Day".to_string()];
        let focus = HashSet::from(["QQQ".to_string()]);
        let mut cursor = 0usize;

        let selected = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &focus,
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            1,
            1,
            1,
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].symbol, "QQQ");
        assert_eq!(selected[0].bucket, AlpacaSyncBucket::Missing);
        assert_eq!(cursor, 1);
    }

    #[test]
    fn select_alpaca_sync_workset_rotating_walks_all_symbols_mn1_before_lower_timeframes() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "QQQ".to_string()];
        let timeframes = vec![
            "1Min".to_string(),
            "1Week".to_string(),
            "1Month".to_string(),
        ];
        let mut cursor = 0usize;

        let first = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            2,
            0,
            2,
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
        );
        assert_eq!(
            first
                .iter()
                .map(|c| (&c.symbol, &c.timeframe))
                .collect::<Vec<_>>(),
            vec![
                (&"AAPL".to_string(), &"1Month".to_string()),
                (&"MSFT".to_string(), &"1Month".to_string()),
            ]
        );
        assert_eq!(cursor, 2);

        let pending: HashSet<String> = first
            .iter()
            .map(|c| alpaca_fetch_key(&c.symbol, &c.timeframe))
            .collect();
        let second = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            &HashMap::new(),
            &pending,
            2,
            0,
            2,
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
        );
        assert_eq!(
            second
                .iter()
                .map(|c| (&c.symbol, &c.timeframe))
                .collect::<Vec<_>>(),
            vec![(&"QQQ".to_string(), &"1Month".to_string()),]
        );
        assert_eq!(cursor, 1);

        let pending: HashSet<String> = pending
            .into_iter()
            .chain(
                second
                    .iter()
                    .map(|c| alpaca_fetch_key(&c.symbol, &c.timeframe)),
            )
            .collect();
        let third = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            &HashMap::new(),
            &pending,
            2,
            0,
            2,
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
        );
        assert_eq!(
            third
                .iter()
                .map(|c| (&c.symbol, &c.timeframe))
                .collect::<Vec<_>>(),
            vec![
                (&"MSFT".to_string(), &"1Week".to_string()),
                (&"QQQ".to_string(), &"1Week".to_string()),
            ]
        );
        assert_eq!(cursor, 0);
    }

    #[test]
    fn select_alpaca_sync_workset_rotating_advances_cursor_by_actual_tail_window() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL", "MSFT", "QQQ", "SPY", "TSLA"]
            .into_iter()
            .map(String::from)
            .collect::<Vec<_>>();
        let timeframes = vec!["1Month".to_string()];
        let mut state = HashMap::new();
        for symbol in ["AAPL", "MSFT", "QQQ", "SPY"] {
            state.insert(
                (symbol.to_string(), "1Month".to_string()),
                SyncCacheState {
                    last_bar_ts_s: now_s,
                    write_ts_s: now_s,
                    bar_count: i64::from(u32::MAX),
                },
            );
        }
        let mut cursor = 0usize;

        let selected = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &state,
            &HashSet::new(),
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            1,
            0,
            4,
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].symbol, "TSLA");
        assert_eq!(selected[0].timeframe, "1Month");
        assert_eq!(
            cursor, 0,
            "tail windows smaller than scan_limit should not advance the cursor past already-scanned symbols"
        );
    }

    #[test]
    fn select_alpaca_sync_workset_rotating_keeps_global_high_tf_priority_across_slices() {
        let now_s = 1_700_000_000i64;
        let symbols = vec![
            "AAPL".to_string(),
            "MSFT".to_string(),
            "QQQ".to_string(),
            "TSLA".to_string(),
        ];
        let timeframes = vec!["1Month".to_string(), "1Week".to_string()];
        let mut state = HashMap::new();
        for symbol in ["AAPL", "MSFT"] {
            state.insert(
                (symbol.to_string(), "1Month".to_string()),
                SyncCacheState {
                    last_bar_ts_s: now_s,
                    write_ts_s: now_s,
                    bar_count: i64::from(u32::MAX),
                },
            );
        }
        let mut cursor = 0usize;

        let selected = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &state,
            &HashSet::new(),
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            2,
            0,
            2,
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(
            selected
                .iter()
                .map(|c| (&c.symbol, &c.timeframe))
                .collect::<Vec<_>>(),
            vec![
                (&"QQQ".to_string(), &"1Month".to_string()),
                (&"TSLA".to_string(), &"1Month".to_string()),
            ],
            "later missing 1Month symbols must be scheduled before lower-TF work in an already-complete cursor slice"
        );
        assert_eq!(cursor, 0);
    }

    #[test]
    fn select_alpaca_sync_workset_rotating_gives_focus_the_full_batch() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "QQQ".to_string()];
        let timeframes = vec!["15Min".to_string()];
        let focus = HashSet::from(["MSFT".to_string(), "QQQ".to_string()]);
        let mut cursor = 0usize;

        let selected = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &focus,
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            3,
            1,
            3,
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 3);
        assert_eq!(selected[0].symbol, "MSFT");
        assert_eq!(selected[1].symbol, "QQQ");
        assert!(selected[0].focus);
        assert!(selected[1].focus);
    }

    #[test]
    fn select_alpaca_sync_workset_rotating_does_not_foreground_m1_m5() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "QQQ".to_string()];
        let timeframes = vec!["1Min".to_string(), "5Min".to_string()];
        let focus = HashSet::from(["QQQ".to_string()]);
        let mut cursor = 0usize;

        let selected = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &focus,
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            1,
            1,
            1,
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].symbol, "AAPL");
        assert!(!selected[0].focus);
        assert_eq!(cursor, 1);
    }

    #[test]
    fn foreground_stale_refresh_is_due_after_one_timeframe_period() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string(), "MSFT".to_string()];
        let timeframes = vec!["1Min".to_string()];
        let state_map = HashMap::from([
            (
                ("AAPL".to_string(), "1Min".to_string()),
                SyncCacheState {
                    last_bar_ts_s: now_s - 65,
                    write_ts_s: now_s - 60,
                    bar_count: 50_000,
                },
            ),
            (
                ("MSFT".to_string(), "1Min".to_string()),
                SyncCacheState {
                    last_bar_ts_s: now_s - 65,
                    write_ts_s: now_s - 60,
                    bar_count: 50_000,
                },
            ),
        ]);
        let focus = HashSet::from(["AAPL".to_string()]);

        let selected = select_alpaca_sync_candidates(
            &symbols,
            &timeframes,
            &state_map,
            &focus,
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            2,
            now_s,
            |_| None,
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].symbol, "AAPL");
        assert_eq!(selected[0].bucket, AlpacaSyncBucket::Stale);
        assert!(selected[0].focus);
    }

    #[test]
    fn select_alpaca_sync_workset_rotating_skips_pending_without_advancing_priority() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string(), "MSFT".to_string(), "QQQ".to_string()];
        let timeframes = vec!["1Min".to_string(), "1Month".to_string()];
        let pending = HashSet::from([alpaca_fetch_key("AAPL", "1Month")]);
        let mut cursor = 0usize;

        let selected = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &HashMap::new(),
            &HashSet::new(),
            &HashSet::new(),
            &HashMap::new(),
            &pending,
            2,
            0,
            3,
            &mut cursor,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(
            selected
                .iter()
                .map(|c| (&c.symbol, &c.timeframe))
                .collect::<Vec<_>>(),
            vec![
                (&"MSFT".to_string(), &"1Month".to_string()),
                (&"QQQ".to_string(), &"1Month".to_string()),
            ]
        );
        assert_eq!(cursor, 0);
    }

    #[test]
    fn high_timeframe_backfill_preempts_lower_timeframe_stale_refresh() {
        let now_s = 1_700_000_000i64;
        let symbols = vec!["AAPL".to_string()];
        let timeframes = vec!["1Hour".to_string(), "1Month".to_string()];
        let state_map = HashMap::from([
            (
                ("AAPL".to_string(), "1Month".to_string()),
                SyncCacheState {
                    last_bar_ts_s: now_s - 60,
                    write_ts_s: now_s - 60,
                    bar_count: 10,
                },
            ),
            (
                ("AAPL".to_string(), "1Hour".to_string()),
                SyncCacheState {
                    last_bar_ts_s: now_s - 25 * 3600,
                    write_ts_s: now_s - 3600,
                    bar_count: 50_000,
                },
            ),
        ]);
        let mut cursor = 0usize;

        let selected = select_alpaca_sync_workset_rotating(
            &symbols,
            &timeframes,
            &state_map,
            &HashSet::new(),
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            1,
            0,
            1,
            &mut cursor,
            now_s,
            |_| Some(100),
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].timeframe, "1Month");
        assert_eq!(selected[0].bucket, AlpacaSyncBucket::Backfill);
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
            &HashSet::new(),
            &HashMap::new(),
            &HashSet::new(),
            1,
            now_s,
            alpaca_sync_target_bars,
        );

        assert_eq!(selected.len(), 1);
        assert_eq!(selected[0].bucket, AlpacaSyncBucket::Stale);
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
        let backfill_complete = HashMap::from([(
            alpaca_fetch_key("LUMN", "1Month"),
            AlpacaBackfillCompletePair {
                symbol: "LUMN".to_string(),
                timeframe: "1Month".to_string(),
                marked_at: now_s,
                bar_count: 70,
                target_bars: u32::MAX as i64,
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
            alpaca_sync_target_bars,
        );

        assert!(selected.is_empty());
    }
}
