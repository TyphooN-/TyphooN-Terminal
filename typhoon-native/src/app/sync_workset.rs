//! Broker-agnostic sync scheduler primitives.
//!
//! Houses the shared bar-sync workset selection logic that every broker
//! integration plugs into: timeframe normalization, the candidate bucket
//! (Missing / Stale / Backfill), focus-vs-background ordering, and the
//! rotating high-TF-first ring used by Alpaca and Kraken
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

const LOW_TF_RESERVE_TIMEFRAMES: [&str; 4] = ["1Min", "5Min", "15Min", "30Min"];

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
pub(crate) struct AlpacaBackfillCompletePair {
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

/// Order candidates by priority tier, then high-timeframe-first, so the
/// coverage-first path still fills gaps in tier order (MTF grid / focus before
/// the rest) instead of by raw bucket order. `tier_a..tier_d` are symbol sets
/// in descending priority; a symbol in none lands in the lowest tier.
#[cfg(test)]
fn sort_candidates_by_priority_then_timeframe(
    candidates: Vec<AlpacaSyncCandidate>,
    tier_a: &HashSet<String>,
    tier_b: &HashSet<String>,
    tier_c: &HashSet<String>,
    tier_d: &HashSet<String>,
) -> Vec<AlpacaSyncCandidate> {
    let tier_rank = |symbol: &str| -> u8 {
        let key = normalize_market_data_symbol(symbol).replace('/', "");
        if tier_a.contains(&key) {
            0
        } else if tier_b.contains(&key) {
            1
        } else if tier_c.contains(&key) {
            2
        } else if tier_d.contains(&key) {
            3
        } else {
            4
        }
    };
    let mut ordered = candidates;
    ordered.sort_by(|a, b| {
        tier_rank(&a.symbol)
            .cmp(&tier_rank(&b.symbol))
            .then(
                sync_timeframe_high_first_sort_key(&a.timeframe)
                    .cmp(&sync_timeframe_high_first_sort_key(&b.timeframe)),
            )
            .then(b.focus.cmp(&a.focus))
            .then(b.score.cmp(&a.score))
            .then(a.symbol.cmp(&b.symbol))
    });
    ordered
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

    let mut selected: Vec<AlpacaSyncCandidate> = Vec::with_capacity(batch_size);
    let mut staged_pending = pending_fetches.clone();

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
        // Avoid redundant clone of focus set; pass directly for tier ranking.
        let ordered_coverage = sort_candidates_by_priority_then_timeframe(
            coverage,
            focus_symbols,
            focus_symbols,
            &std::collections::HashSet::new(),
            &std::collections::HashSet::new(),
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

#[allow(clippy::too_many_arguments)]
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
    is_dispatch_blocked: &dyn Fn(&str, &str) -> bool,
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
        is_dispatch_blocked,
    )
}

/// `is_dispatch_blocked(raw_symbol, tf)` mirrors the per-lane queue-time gates the
/// selector cannot otherwise see (chiefly the `is_fetch_on_cooldown` window, which
/// spans half a TF period — 3.5 DAYS for 1Week, 15 for 1Month). Without it, a
/// timeframe whose remaining candidates are all cooldown-armed still fills every
/// batch with them; the queue path then drops 100% at dispatch, and the strict
/// high-TF-first descent never reaches the lower timeframes. That is exactly the
/// overnight "lane goes silent for 8h while 1Day sits at 1.8%" wedge: blocked
/// candidates must neither consume batch slots nor hold the TF descent hostage.
#[allow(clippy::too_many_arguments)]
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
    is_dispatch_blocked: &dyn Fn(&str, &str) -> bool,
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
                    is_dispatch_blocked,
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
                    is_dispatch_blocked,
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

/// Select a bounded low-timeframe reserve batch in low→high order. The main
/// broad scheduler remains high-TF-first, but full-tilt catch-up needs a small
/// side lane so M1/M5/M15/M30 rows don't sit at ~0% for hours while H1+ depth is
/// still converging. This uses the same tombstone, pending, cooldown, and
/// backfill-complete gates as the main selector; only timeframe order changes.
#[allow(clippy::too_many_arguments)]
pub(super) fn select_low_timeframe_sync_reserve_rotating(
    symbols: &[String],
    timeframes: &[String],
    state_map: &HashMap<(String, String), SyncCacheState>,
    focus_symbols: &HashSet<String>,
    no_data_keys: &HashSet<String>,
    backfill_complete_pairs: &HashMap<String, AlpacaBackfillCompletePair>,
    pending_fetches: &HashSet<String>,
    batch_size: usize,
    background_scan_limit: usize,
    cursor: &mut usize,
    now_s: i64,
    background_stale_periods: i64,
    target_bars_for_tf: fn(&str) -> Option<u32>,
    is_dispatch_blocked: &dyn Fn(&str, &str) -> bool,
) -> Vec<AlpacaSyncCandidate> {
    if batch_size == 0 || symbols.is_empty() || timeframes.is_empty() {
        return Vec::new();
    }
    let requested: HashSet<&'static str> = timeframes
        .iter()
        .filter_map(|tf| normalize_sync_timeframe_key(tf))
        .collect();
    let ordered_timeframes: Vec<&'static str> = LOW_TF_RESERVE_TIMEFRAMES
        .iter()
        .copied()
        .filter(|tf| requested.contains(tf))
        .collect();
    if ordered_timeframes.is_empty() {
        return Vec::new();
    }

    let total_symbols = symbols.len();
    let background_scan_limit = background_scan_limit.max(batch_size).min(total_symbols);
    let symbol_start = *cursor % total_symbols;

    for tf in ordered_timeframes {
        let mut staged_selected = pending_fetches.clone();
        let mut missing: Vec<AlpacaSyncCandidate> = Vec::with_capacity(batch_size);
        let mut stale: Vec<AlpacaSyncCandidate> = Vec::with_capacity(batch_size);
        let mut backfill: Vec<AlpacaSyncCandidate> = Vec::with_capacity(batch_size);

        let mut foreground_symbols: Vec<&str> = focus_symbols.iter().map(String::as_str).collect();
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
                is_dispatch_blocked,
                &mut missing,
                &mut stale,
                &mut backfill,
            );
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
                    is_dispatch_blocked,
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
    is_dispatch_blocked: &dyn Fn(&str, &str) -> bool,
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
    // Probed only for actionable candidates (post-classify) so the common
    // no-work case stays allocation-free. Pass the RAW symbol: each lane's
    // predicate applies its own queue-fn normalization (pair/futures/market)
    // so the probe key matches what `mark_fetch_queued` recorded.
    if is_dispatch_blocked(symbol, tf) {
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
mod tests;
