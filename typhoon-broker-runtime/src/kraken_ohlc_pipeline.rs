//! Lower-layer Kraken WS v2 OHLC pipeline runtime.
//!
//! Native scheduling decides when to start streams; this module owns streamer spawn,
//! channel backpressure, coalescing, cache writes, and WS-fresh commit reporting.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use typhoon_engine::broker::kraken::{
    KrakenOhlcStreamerEvent, KrakenWsOhlcBar, kraken_ws_bar_to_json,
    kraken_ws_interval_to_tf_label, kraken_ws_symbol_to_cache_key, run_ohlc_snapshot_sweep_once,
    run_ohlc_streamer_with_snapshot, ws_bar_is_closed,
};
use typhoon_engine::core::cache::SqliteCache;

pub fn format_xstock_ws_symbol(symbol: &str) -> Option<String> {
    let bare = symbol
        .trim()
        .trim_end_matches(".EQ")
        .trim_end_matches(".eq")
        .to_ascii_uppercase();
    if bare.is_empty() || bare.contains('/') {
        return None;
    }
    Some(format!("{bare}x/USD"))
}

pub fn kraken_ws_bar_cache_target(ws_symbol: &str) -> Option<WsCacheTarget> {
    let trimmed = ws_symbol.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Some((base, quote)) = trimmed.split_once('/') {
        if quote.eq_ignore_ascii_case("USD")
            && base.ends_with('x')
            && base.len() > 1
            && base[..base.len() - 1]
                .chars()
                .all(|c| c.is_ascii_alphanumeric())
        {
            let symbol = base[..base.len() - 1].to_ascii_uppercase();
            return Some(("kraken-equities", symbol));
        }
    }
    let symbol = kraken_ws_symbol_to_cache_key(trimmed);
    if symbol.is_empty() {
        None
    } else {
        Some(("kraken", symbol))
    }
}

/// Coalesce-and-flush cadence for the WS bar writer. Keep this tight so the
/// initial full-universe snapshots mark REST slots WS-fresh almost immediately;
/// closed-bar gating still prevents per-tick rewrites of open buckets.
const WS_BAR_FLUSH_INTERVAL: Duration = Duration::from_secs(1);

/// Bounded channel capacity between WS streamers and the writer. Startup
/// snapshots can otherwise enqueue millions of bars while the writer is busy
/// merging/compressing, which converts a CPU burst into unbounded memory growth.
const WS_BAR_CHANNEL_CAPACITY: usize = 65_536;

/// Maximum raw bars to hold in the writer coalescing buffer before forcing a
/// closed-bar flush. Full-universe Kraken startup snapshots are mostly closed
/// historical bars; waiting only for the wall-clock ticker lets millions of
/// unique buckets accumulate in RAM. This threshold converts snapshot ingress
/// into bounded chunks and backpressures WS readers while cache writes catch up.
const WS_BAR_MAX_BUFFERED_BUCKETS: usize = 16_384;

/// Large full-catalog universes must not start every interval at once: each
/// Kraken OHLC subscription asks for a startup snapshot. 12k+ pairs × 8
/// intervals can manufacture tens of millions of bars in a few seconds and OOM
/// the terminal. Keep full coverage, but stage intervals so each snapshot wave
/// can be parsed, backpressured, and persisted before the next wave starts.
pub const WS_LARGE_UNIVERSE_INTERVAL_STAGGER: Duration = Duration::from_secs(120);
pub const WS_LARGE_UNIVERSE_PAIR_THRESHOLD: usize = 5_000;
/// Maximum grouped `(symbol, timeframe)` merges to process in one blocking task.
/// Keeps startup snapshot persistence in bounded slices so tokio can schedule
/// other blocking work and the writer can report WS-fresh progress incrementally.
const WS_BAR_MAX_GROUPS_PER_BLOCKING_FLUSH: usize = 256;

/// Maximum bars per `merge_bars` call. The cache's merge_bars rewrites the
/// entire compressed blob for the key, so we batch but don't try to flush
/// everything in one go — keeps per-flush latency bounded even for active
/// pairs with many recent buckets.
const WS_BAR_MAX_BARS_PER_KEY: usize = 0; // 0 == unbounded (full-depth merge)

/// Triple committed after a flush: `(typhoon_symbol, tf_label, last_bar_ts_ms)`.
/// Sent to the main loop so it can mark the (symbol, tf) WS-fresh. The symbol
/// is source-local: Spot uses `BTCUSD`, xStocks use bare equity symbols like
/// `AAPL` so the iapi scheduler can see the WS path is fresh.
pub type WsFreshEntry = (String, String, i64);

pub type WsCacheTarget = (&'static str, String);
pub type WsBuffer = HashMap<(String, String, u32, i64), KrakenWsOhlcBar>;
type GroupedWsBarEntry = (
    (String, String, &'static str),
    (Vec<serde_json::Value>, i64),
);
pub type GroupedWsBars = HashMap<(String, String, &'static str), (Vec<serde_json::Value>, i64)>;

pub fn chunk_grouped_ws_bar_entries(
    grouped: GroupedWsBars,
    chunk_size: usize,
) -> Vec<Vec<GroupedWsBarEntry>> {
    let chunk_size = chunk_size.max(1);
    let mut chunks: Vec<Vec<GroupedWsBarEntry>> = Vec::new();
    for entry in grouped {
        if chunks.last().is_none_or(|chunk| chunk.len() >= chunk_size) {
            chunks.push(Vec::with_capacity(chunk_size));
        }
        chunks
            .last_mut()
            .expect("chunk should exist after push")
            .push(entry);
    }
    chunks
}

/// Spawn the full WS OHLC pipeline. Drops zero streamers if `pairs` is
/// empty (Kraken has no subscribe support for "all pairs without listing
/// them"). Caller is expected to drive `pairs` from the existing
/// AssetPairs catalog.
///
/// Returns immediately after spawning; lifecycle is owned by the tokio
/// runtime. Streamers and writer exit cleanly when `bars_rx` is dropped,
/// which happens when the shared cache slot is cleared on shutdown.
pub fn spawn_kraken_ohlc_pipeline(
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    pairs: Vec<String>,
    intervals_min: Vec<u32>,
    commit_tx: mpsc::UnboundedSender<Vec<WsFreshEntry>>,
    status_tx: mpsc::UnboundedSender<KrakenOhlcStreamerEvent>,
) {
    if pairs.is_empty() || intervals_min.is_empty() {
        return;
    }
    let (bar_tx, bar_rx) = mpsc::channel::<KrakenWsOhlcBar>(WS_BAR_CHANNEL_CAPACITY);
    for (interval_min, snapshot, startup_delay) in
        plan_kraken_ws_streamers(pairs.len(), &intervals_min)
    {
        let pairs = pairs.clone();
        let bar_tx = bar_tx.clone();
        let status_tx = status_tx.clone();
        tokio::spawn(async move {
            if !startup_delay.is_zero() {
                tokio::time::sleep(startup_delay).await;
            }
            run_ohlc_streamer_with_snapshot(interval_min, pairs, snapshot, bar_tx, status_tx).await;
        });
    }
    // Drop the original sender so the writer's rx.recv() resolves to None
    // when all streamers exit (otherwise it would hang forever on the
    // sender we held).
    drop(bar_tx);
    tokio::spawn(async move {
        run_ws_bar_writer(shared_cache, bar_rx, commit_tx).await;
    });
}

pub fn spawn_kraken_ohlc_snapshot_sweep(
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    interval_min: u32,
    pairs: Vec<String>,
    commit_tx: mpsc::UnboundedSender<Vec<WsFreshEntry>>,
    status_tx: mpsc::UnboundedSender<KrakenOhlcStreamerEvent>,
    settled_tx: mpsc::UnboundedSender<Result<(u32, usize), String>>,
) {
    if pairs.is_empty() {
        let _ = settled_tx.send(Ok((interval_min, 0)));
        return;
    }
    let pair_count = pairs.len();
    let (bar_tx, bar_rx) = mpsc::channel::<KrakenWsOhlcBar>(WS_BAR_CHANNEL_CAPACITY);
    tokio::spawn(async move {
        run_ws_bar_writer(shared_cache, bar_rx, commit_tx).await;
    });
    tokio::spawn(async move {
        let result = run_ohlc_snapshot_sweep_once(interval_min, pairs, bar_tx, status_tx)
            .await
            .map(|()| (interval_min, pair_count));
        let _ = settled_tx.send(result);
    });
}

pub fn kraken_ws_should_request_initial_snapshot(pair_count: usize) -> bool {
    pair_count < WS_LARGE_UNIVERSE_PAIR_THRESHOLD
}

/// `true` for the timeframes whose Kraken WS startup snapshot is *bounded* even
/// across the full xStocks catalog: 1Hour and up. xStocks are newly-listed
/// tokenized equities, so their hourly/4-hour/daily/weekly history is at most a
/// few hundred bars per symbol — safe to snapshot for all ~12k pairs. The
/// sub-hour intervals (1Min/5Min/15Min/30Min) can each carry hundreds-to-tens-
/// of-thousands of bars per symbol; a full-catalog snapshot of those is the OOM
/// risk that `07a1ce3` disabled, so for large universes they stay live-only and
/// fill once the market is trading.
fn kraken_ws_interval_is_bounded_snapshot_tf(interval_min: u32) -> bool {
    interval_min >= 60
}

/// Plan the per-interval streamer schedule as `(interval_min, request_snapshot,
/// startup_delay)`.
///
/// Small universes (Kraken Spot, < `WS_LARGE_UNIVERSE_PAIR_THRESHOLD` pairs)
/// snapshot every interval immediately — this is what gets Spot to ~96% synced,
/// because the snapshot delivers each pair's recent history on subscribe.
///
/// Large universes (the full xStocks catalog) would OOM on a full 1Min snapshot
/// burst, so only the bounded high-timeframe snapshots (1Hour…1Week) are
/// requested; the low timeframes subscribe live-only and fill once the market is
/// trading. The snapshot waves are staggered — highest timeframe (smallest
/// payload) first — so each drains under the writer's backpressure before the
/// next begins. Live-only intervals carry no startup burst, so they start
/// immediately regardless of the stagger.
pub fn plan_kraken_ws_streamers(
    pairs_len: usize,
    intervals_min: &[u32],
) -> Vec<(u32, bool, Duration)> {
    if kraken_ws_should_request_initial_snapshot(pairs_len) {
        return intervals_min
            .iter()
            .map(|&interval_min| (interval_min, true, Duration::ZERO))
            .collect();
    }
    // Live-only low timeframes: no snapshot, no startup burst, start now.
    // M1/M5 (and focused MTF low-TF) use public trades WS for live forming + WS-fresh.
    // This coexists with bounded high-TF snapshots (highest-first staggered) for full-universe.
    let mut plan: Vec<(u32, bool, Duration)> = intervals_min
        .iter()
        .copied()
        .filter(|&interval_min| !kraken_ws_interval_is_bounded_snapshot_tf(interval_min))
        .map(|interval_min| (interval_min, false, Duration::ZERO))
        .collect();
    // Bounded high-timeframe snapshots: highest TF first (smallest payload), one
    // staggered wave each so the writer can persist before the next lands.
    let mut snapshot_intervals: Vec<u32> = intervals_min
        .iter()
        .copied()
        .filter(|&interval_min| kraken_ws_interval_is_bounded_snapshot_tf(interval_min))
        .collect();
    snapshot_intervals.sort_unstable_by(|a, b| b.cmp(a));
    for (wave, interval_min) in snapshot_intervals.into_iter().enumerate() {
        plan.push((
            interval_min,
            true,
            WS_LARGE_UNIVERSE_INTERVAL_STAGGER * wave as u32,
        ));
    }
    plan
}

/// Drain the merged bar channel, buffer by (symbol, interval, bucket),
/// flush every [`WS_BAR_FLUSH_INTERVAL`]. Exits when the channel closes.
///
/// Only **closed** bars are flushed to the cache — open in-progress buckets
/// stay buffered (last-write-wins) until their interval rolls over. This is
/// the load-bearing perf fix for the UI lag the WS feed introduced: an
/// active 1Min pair gets dozens of per-tick updates to the same open bucket,
/// and persisting each one runs [`SqliteCache::merge_bars`] which decompresses
/// the full history, re-sorts, re-serialises, and recompresses the entire
/// blob. At high base zstd levels, ~1500 pairs × 8 intervals saturated every
/// CPU core and starved egui's render thread. Deferring to bar close drops the
/// steady-state write rate by ~60× (1 merge per closed bar per pair rather
/// than per WS tick) while preserving the freshness semantic the REST
/// scheduler relies on — `kraken_ws_fresh_until` still gets updated for each
/// (symbol, tf) on close, which is exactly when staleness checks care.
async fn run_ws_bar_writer(
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    mut bar_rx: mpsc::Receiver<KrakenWsOhlcBar>,
    commit_tx: mpsc::UnboundedSender<Vec<WsFreshEntry>>,
) {
    // (symbol_cache_key, interval_min, interval_begin_ms) -> bar
    // Last-write-wins for the same bucket, which is exactly the semantic
    // Kraken's WS uses: each new update for the open bar supersedes the
    // previous one until interval_begin rolls.
    let mut buffer: WsBuffer = HashMap::new();

    let mut flush_ticker = tokio::time::interval(WS_BAR_FLUSH_INTERVAL);
    flush_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    flush_ticker.tick().await; // skip the immediate first fire

    loop {
        tokio::select! {
            maybe_bar = bar_rx.recv() => {
                match maybe_bar {
                    Some(bar) => {
                        let Some((cache_source, cache_symbol)) = kraken_ws_bar_cache_target(&bar.symbol) else {
                            continue;
                        };
                        buffer.insert(
                            (
                                cache_source.to_string(),
                                cache_symbol,
                                bar.interval_min,
                                bar.interval_begin_ms,
                            ),
                            bar,
                        );
                        if buffer.len() >= WS_BAR_MAX_BUFFERED_BUCKETS {
                            let now_ms = chrono::Utc::now().timestamp_millis();
                            let (to_flush, remaining) =
                                partition_closed_bars(std::mem::take(&mut buffer), now_ms);
                            buffer = remaining;
                            if !to_flush.is_empty() {
                                flush_ws_bars(&shared_cache, &commit_tx, to_flush).await;
                            }
                        }
                    }
                    None => break, // every streamer dropped its sender → shut down
                }
            }
            _ = flush_ticker.tick() => {
                if buffer.is_empty() {
                    continue;
                }
                let now_ms = chrono::Utc::now().timestamp_millis();
                let (to_flush, remaining) =
                    partition_closed_bars(std::mem::take(&mut buffer), now_ms);
                buffer = remaining;
                if !to_flush.is_empty() {
                    flush_ws_bars(&shared_cache, &commit_tx, to_flush).await;
                }
            }
        }
    }
    // Final drain on shutdown: flush everything regardless of close state.
    // We're going down, so persisting the latest known value of an open bar
    // is strictly better than dropping it on the floor.
    if !buffer.is_empty() {
        flush_ws_bars(&shared_cache, &commit_tx, std::mem::take(&mut buffer)).await;
    }
}

/// Split the writer buffer into (closed-bars-to-flush, still-open-bars-to-keep).
/// A bar is "closed" once its bucket end is at or before `now_ms`; open bars
/// stay in the buffer so the next tick's update can supersede them (last-write
/// -wins) and only the eventual final close value reaches the cache.
pub fn partition_closed_bars(buffer: WsBuffer, now_ms: i64) -> (WsBuffer, WsBuffer) {
    buffer
        .into_iter()
        .partition(|((_, _, interval_min, interval_begin_ms), _)| {
            ws_bar_is_closed(*interval_min, *interval_begin_ms, now_ms)
        })
}

/// Group buffered bars by `(typhoon_symbol, tf_label)`, merge each group
/// into the cache in one `merge_bars` call, and report fresh entries to
/// the main loop.
async fn flush_ws_bars(
    shared_cache: &Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    commit_tx: &mpsc::UnboundedSender<Vec<WsFreshEntry>>,
    buffer: WsBuffer,
) {
    let Some(cache) = shared_cache.read().ok().and_then(|g| g.clone()) else {
        return;
    };

    // Group bars by their target cache key + track the newest bucket ts so
    // we can report it back as the WS-fresh anchor. Startup snapshots can
    // produce ~12k groups; process them in bounded blocking chunks below.
    let mut grouped: GroupedWsBars = HashMap::new();
    for ((cache_source, cache_symbol, interval_min, interval_begin_ms), bar) in buffer {
        let Some(tf_label) = kraken_ws_interval_to_tf_label(interval_min) else {
            continue;
        };
        let json = kraken_ws_bar_to_json(&bar);
        let entry = grouped
            .entry((cache_source, cache_symbol, tf_label))
            .or_insert_with(|| (Vec::new(), 0));
        entry.0.push(json);
        if interval_begin_ms > entry.1 {
            entry.1 = interval_begin_ms;
        }
    }

    if grouped.is_empty() {
        return;
    }

    for chunk in chunk_grouped_ws_bar_entries(grouped, WS_BAR_MAX_GROUPS_PER_BLOCKING_FLUSH) {
        let cache_clone = cache.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut committed: Vec<WsFreshEntry> = Vec::with_capacity(chunk.len());
            for ((cache_source, cache_symbol, tf_label), (bars, last_bucket_ms)) in chunk {
                if bars.is_empty() {
                    continue;
                }
                let key = format!("{cache_source}:{cache_symbol}:{tf_label}");
                let bars_json = match serde_json::to_string(&bars) {
                    Ok(j) => j,
                    Err(_) => continue,
                };
                if cache_clone
                    .merge_bars_fast(&key, &bars_json, WS_BAR_MAX_BARS_PER_KEY)
                    .is_ok()
                {
                    committed.push((cache_symbol, tf_label.to_string(), last_bucket_ms));
                }
            }
            committed
        })
        .await;

        if let Ok(committed) = result {
            if !committed.is_empty() {
                let _ = commit_tx.send(committed);
            }
        }
        // Cooperate with the runtime between startup snapshot chunks instead
        // of monopolizing the writer task until every key has been merged.
        tokio::task::yield_now().await;
    }
}
