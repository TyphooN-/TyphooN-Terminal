//! Native-side runtime for the Kraken WS v2 OHLC streamers.
//!
//! The engine crate publishes the protocol layer (subscribe frames, message
//! parsing, the per-interval streamer task). This module owns the rest:
//!
//!  * Spawn one streamer per interval Kraken serves (1Min … 1Week).
//!  * Drain every streamer's bar channel into a single buffered writer.
//!  * Coalesce same-bucket updates (one bar can be re-emitted many times
//!    per second as ticks land) and flush in batches every
//!    `WS_BAR_FLUSH_INTERVAL`, so the SqliteCache sees one merge per
//!    (symbol, timeframe) per flush instead of one per WS tick.
//!  * Notify the main app loop after every flush with the (symbol,
//!    timeframe, last_bar_ts_ms) triples that just committed, so the
//!    REST scheduler can mark them WS-fresh and skip refetch.

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use typhoon_engine::broker::kraken::{
    KRAKEN_WS_OHLC_INTERVALS_MIN, KrakenOhlcStreamerEvent, KrakenWsOhlcBar, kraken_ws_bar_to_json,
    kraken_ws_interval_to_tf_label, kraken_ws_symbol_to_cache_key, run_ohlc_snapshot_sweep_once,
    run_ohlc_streamer_with_snapshot, ws_bar_is_closed,
};
use typhoon_engine::core::cache::SqliteCache;

use super::{BrokerCmd, LogEntry, TyphooNApp};

impl TyphooNApp {
    /// Kick off the Kraken WS OHLC pipeline for every Kraken Spot pair that can
    /// be represented on Kraken's public OHLC WebSocket.
    ///
    /// Kraken REST is still the deep-history/backfill source, but the public WS
    /// can deliver far more fresh bars per second than the paced REST endpoint.
    /// Do not narrow Spot to active/watchlist symbols: the product goal is a
    /// fully synced Kraken Spot dataset wherever Kraken exposes the pair on WS.
    /// xStocks get the same treatment now: subscribe the full Kraken Equities
    /// catalog on WS as the fastest fresh-bar source, while REST iapi remains
    /// paced/demand-biased for deep history and Cloudflare safety. The writer
    /// side gates cache persistence to closed bars and uses the fast zstd path
    /// so startup snapshots/fresh closes do not block egui.
    ///
    /// Idempotent and additive: already-streamed symbols are tracked so Kraken
    /// AssetPairs can start Spot immediately, then full-catalog xStocks can be
    /// added later without duplicating existing streamers.
    pub(super) fn maybe_start_kraken_ws_ohlc(&mut self) -> bool {
        if !self.kraken_enabled || !self.kraken_ws_ohlc_enabled {
            return false;
        }
        let desired = build_kraken_ws_subscribe_symbols_for_app(
            &self.kraken_pairs,
            &self.kraken_equity_universe_symbols,
            &self.kraken_equity_demand_symbols(),
            self.kraken_scrape_xstocks,
        );
        if desired.is_empty() {
            tracing::debug!("Kraken WS OHLC deferred: no WS-mappable Kraken pairs");
            return false;
        }
        let pairs: Vec<String> = desired
            .into_iter()
            .filter(|pair| !self.kraken_ws_ohlc_streamed_pairs.contains(pair))
            .collect();
        let intervals_min = enabled_kraken_ws_ohlc_intervals(&self.enabled_sync_timeframes);
        if intervals_min.is_empty() {
            tracing::debug!("Kraken WS OHLC deferred: no enabled WS OHLC intervals");
            return false;
        }
        if pairs.is_empty() {
            return false;
        }
        let count = pairs.len();
        let interval_count = intervals_min.len();
        let total_after = self.kraken_ws_ohlc_streamed_pairs.len() + count;
        for pair in &pairs {
            self.kraken_ws_ohlc_streamed_pairs.insert(pair.clone());
        }
        let _ = self.broker_tx.send(BrokerCmd::KrakenStartOhlcStreamers {
            pairs,
            intervals_min,
        });
        self.kraken_ws_ohlc_started = true;
        self.log.push_back(LogEntry::info(format!(
            "Kraken WS OHLC: streaming {count} additional pairs ({total_after} total) × {interval_count} enabled intervals",
        )));
        true
    }

    pub(super) fn maybe_schedule_kraken_ws_ohlc_snapshot_sweep(&mut self) -> bool {
        if !self.kraken_enabled
            || !self.kraken_ws_ohlc_enabled
            || !self.kraken_scrape_xstocks
            || self.kraken_ws_ohlc_snapshot_sweep_in_flight
        {
            return false;
        }
        let now = std::time::Instant::now();
        // When the xStocks market is shut for the weekend, OHLC bars don't change, so
        // re-sweeping the full token catalog every 10s is pure waste that spikes UI
        // frames (the overnight stall storm). Back off to a slow heartbeat; the normal
        // 10s cadence resumes automatically on the Sunday 8pm ET reopen.
        let cadence = if super::app_runtime_support::kraken_xstocks_weekend_closed_now() {
            KRAKEN_WS_SNAPSHOT_SWEEP_CLOSED_CADENCE
        } else {
            KRAKEN_WS_SNAPSHOT_SWEEP_CADENCE
        };
        if now.duration_since(self.kraken_ws_ohlc_snapshot_sweep_last_schedule) < cadence {
            return false;
        }
        // Scope the sweep to WS-tokenized xStocks (the `{SYM}x/USD` pairs that
        // actually exist on Kraken's WS v2), not the full ~12k iapi catalog — the
        // catalog is ~99% non-WS Securities, so subscribing it was ~99% phantom
        // and starved the real tokens. Full catalog breadth still comes from the
        // Alpaca/Yahoo lanes + demand-scoped iapi (see ADR-112).
        let catalog = self.kraken_equity_ws_sweep_symbols();
        let intervals_min =
            enabled_kraken_ws_ohlc_snapshot_sweep_intervals(&self.enabled_sync_timeframes);
        if intervals_min.is_empty() {
            return false;
        }
        let Some(batch) = next_kraken_ws_snapshot_sweep_batch(
            &catalog,
            &mut self.kraken_ws_ohlc_snapshot_sweep_cursor,
            KRAKEN_WS_SNAPSHOT_SWEEP_BATCH_SIZE,
            &intervals_min,
        ) else {
            return false;
        };
        let pair_count = batch.pairs.len();
        self.kraken_ws_ohlc_snapshot_sweep_in_flight = true;
        self.kraken_ws_ohlc_snapshot_sweep_last_schedule = now;
        let interval_min = batch.interval_min;
        let _ = self.broker_tx.send(BrokerCmd::KrakenOhlcSnapshotSweep {
            interval_min,
            pairs: batch.pairs,
        });
        let tf = kraken_ws_interval_to_tf_label(interval_min).unwrap_or("?");
        self.log.push_back(LogEntry::info(format!(
            "Kraken WS OHLC snapshot sweep: queued {pair_count} xStocks for {tf}"
        )));
        true
    }
}

/// Transform the `(pair_name, display_symbol)` tuples that Kraken's
/// `/0/public/AssetPairs` REST call returns into the ws-friendly
/// `BASE/QUOTE` format the WS v2 channel expects. Prefers the display
/// symbol when it already contains a slash, otherwise inserts one between
/// the base and quote derived from the legacy 8-char pair name. Deduped
/// via BTreeSet for stable ordering and O(log n) inserts.
#[cfg(test)]
fn build_kraken_ws_subscribe_symbols(pairs: &[(String, String)]) -> Vec<String> {
    build_kraken_ws_subscribe_symbols_for_app(pairs, &[], &[], false)
}

pub(super) fn build_kraken_ws_subscribe_symbols_for_app(
    spot_pairs: &[(String, String)],
    xstock_catalog_symbols: &[String],
    xstock_demand_symbols: &[String],
    include_xstocks: bool,
) -> Vec<String> {
    let mut out = std::collections::BTreeSet::new();
    for (pair_name, display) in spot_pairs {
        if let Some(formatted) = format_ws_symbol(pair_name, display) {
            out.insert(formatted);
        }
    }
    if include_xstocks {
        // Stream only the demand set (held / open chart / watchlist xStocks),
        // never the full ~12k reference catalog. Subscribing the whole catalog
        // across 8 intervals overwhelmed a single WS v2 connection — constant
        // "connection reset without closing handshake" churn plus snapshot write
        // storms that stalled egui for seconds. Catalog breadth is carried by the
        // paced snapshot sweep below and by batched Alpaca/Yahoo history lanes.
        let _ = xstock_catalog_symbols;
        for symbol in xstock_demand_symbols {
            if let Some(formatted) = format_xstock_ws_symbol(symbol) {
                out.insert(formatted);
            }
        }
    }
    out.into_iter().collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct KrakenWsSnapshotSweepBatch {
    interval_min: u32,
    pairs: Vec<String>,
}

const KRAKEN_WS_SNAPSHOT_SWEEP_INTERVALS_HIGH_FIRST: [u32; 8] = [
    10080, // 1Week
    1440,  // 1Day
    240,   // 4Hour
    60,    // 1Hour
    30,    // 30Min
    15,    // 15Min
    5,     // 5Min
    1,     // 1Min
];

fn enabled_kraken_ws_ohlc_intervals(enabled_sync_timeframes: &BTreeSet<String>) -> Vec<u32> {
    KRAKEN_WS_OHLC_INTERVALS_MIN
        .iter()
        .copied()
        .filter(|&interval_min| {
            kraken_ws_interval_to_tf_label(interval_min)
                .is_some_and(|tf| enabled_sync_timeframes.contains(tf))
        })
        .collect()
}

fn enabled_kraken_ws_ohlc_snapshot_sweep_intervals(
    enabled_sync_timeframes: &BTreeSet<String>,
) -> Vec<u32> {
    KRAKEN_WS_SNAPSHOT_SWEEP_INTERVALS_HIGH_FIRST
        .iter()
        .copied()
        .filter(|&interval_min| {
            kraken_ws_interval_to_tf_label(interval_min)
                .is_some_and(|tf| enabled_sync_timeframes.contains(tf))
        })
        .collect()
}

fn next_kraken_ws_snapshot_sweep_batch(
    catalog_symbols: &[String],
    cursor: &mut usize,
    batch_size: usize,
    intervals_high_first: &[u32],
) -> Option<KrakenWsSnapshotSweepBatch> {
    let batch_size = batch_size.max(1);
    if intervals_high_first.is_empty() {
        *cursor = 0;
        return None;
    }
    let pairs: Vec<String> = catalog_symbols
        .iter()
        .filter_map(|symbol| format_xstock_ws_symbol(symbol))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if pairs.is_empty() {
        *cursor = 0;
        return None;
    }
    let batches_per_interval = pairs.len().div_ceil(batch_size).max(1);
    let total_steps = batches_per_interval * intervals_high_first.len();
    let step = *cursor % total_steps;
    let interval_idx = step / batches_per_interval;
    let batch_idx = step % batches_per_interval;
    let start = batch_idx * batch_size;
    let end = (start + batch_size).min(pairs.len());
    *cursor = (step + 1) % total_steps;
    Some(KrakenWsSnapshotSweepBatch {
        interval_min: intervals_high_first[interval_idx],
        pairs: pairs[start..end].to_vec(),
    })
}

fn format_xstock_ws_symbol(symbol: &str) -> Option<String> {
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

fn kraken_ws_bar_cache_target(ws_symbol: &str) -> Option<WsCacheTarget> {
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

fn format_ws_symbol(pair_name: &str, display: &str) -> Option<String> {
    let display = display.trim();
    if display.contains('/') && display.len() <= 32 {
        return Some(display.to_ascii_uppercase());
    }
    // Fall back to splitting the legacy pair name. Kraken uses X- and Z-
    // prefixed base/quote codes for legacy assets (XXBT/ZUSD); the existing
    // normaliser folds those into the user-facing form. We then split on
    // the common quote suffixes to insert the slash.
    let normalised =
        typhoon_engine::core::kraken::normalize_pair_symbol(pair_name).to_ascii_uppercase();
    if normalised.is_empty() {
        return None;
    }
    pub(crate) const QUOTES: [&str; 12] = [
        "USDG", "USDT", "USDC", "USD", "EUR", "GBP", "CAD", "AUD", "JPY", "CHF", "XBT", "BTC",
    ];
    for quote in QUOTES {
        if let Some(base) = normalised.strip_suffix(quote) {
            if base.is_empty() {
                continue;
            }
            return Some(format!("{base}/{quote}"));
        }
    }
    None
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
const WS_LARGE_UNIVERSE_INTERVAL_STAGGER: Duration = Duration::from_secs(120);
const WS_LARGE_UNIVERSE_PAIR_THRESHOLD: usize = 5_000;
const KRAKEN_WS_SNAPSHOT_SWEEP_BATCH_SIZE: usize = 250;
const KRAKEN_WS_SNAPSHOT_SWEEP_CADENCE: Duration = Duration::from_secs(10);
/// Slow heartbeat used while the xStocks market is closed for the weekend — the
/// catalog OHLC is static, so a 10-minute cadence keeps a minimal catch-up sweep
/// without the per-10s full-catalog storm that spiked UI frames overnight.
const KRAKEN_WS_SNAPSHOT_SWEEP_CLOSED_CADENCE: Duration = Duration::from_secs(600);

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
pub(super) type WsFreshEntry = (String, String, i64);

type WsCacheTarget = (&'static str, String);
type WsBuffer = HashMap<(String, String, u32, i64), KrakenWsOhlcBar>;
type GroupedWsBarEntry = (
    (String, String, &'static str),
    (Vec<serde_json::Value>, i64),
);
type GroupedWsBars = HashMap<(String, String, &'static str), (Vec<serde_json::Value>, i64)>;

fn chunk_grouped_ws_bar_entries(
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
pub(super) fn spawn_kraken_ohlc_pipeline(
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
        run_ws_bar_writer(shared_cache, bar_rx, commit_tx, None).await;
    });
}

pub(super) fn spawn_kraken_ohlc_snapshot_sweep(
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
        run_ws_bar_writer(shared_cache, bar_rx, commit_tx, None).await;
    });
    tokio::spawn(async move {
        let result = run_ohlc_snapshot_sweep_once(interval_min, pairs, bar_tx, status_tx)
            .await
            .map(|()| (interval_min, pair_count));
        let _ = settled_tx.send(result);
    });
}

fn kraken_ws_should_request_initial_snapshot(pair_count: usize) -> bool {
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
fn plan_kraken_ws_streamers(pairs_len: usize, intervals_min: &[u32]) -> Vec<(u32, bool, Duration)> {
    if kraken_ws_should_request_initial_snapshot(pairs_len) {
        return intervals_min
            .iter()
            .map(|&interval_min| (interval_min, true, Duration::ZERO))
            .collect();
    }
    // Live-only low timeframes: no snapshot, no startup burst, start now.
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
    metrics: Option<std::sync::Arc<crate::metrics::MetricsRegistry>>,
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
                // Report real channel saturation to Prometheus
                if let Some(metrics) = &metrics {
                    metrics.set_kraken_ws_bar_channel_stats(
                        WS_BAR_CHANNEL_CAPACITY as f64,
                        bar_rx.len() as f64,
                    );
                }

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
fn partition_closed_bars(buffer: WsBuffer, now_ms: i64) -> (WsBuffer, WsBuffer) {
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

#[cfg(test)]
mod tests;
