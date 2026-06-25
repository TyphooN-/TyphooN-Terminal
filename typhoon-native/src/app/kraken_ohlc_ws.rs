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

use std::collections::BTreeSet;
use std::time::Duration;

#[cfg(test)]
use typhoon_broker_runtime::kraken_ohlc_pipeline::{
    GroupedWsBars, WS_LARGE_UNIVERSE_INTERVAL_STAGGER, WS_LARGE_UNIVERSE_PAIR_THRESHOLD,
    chunk_grouped_ws_bar_entries, kraken_ws_should_request_initial_snapshot, partition_closed_bars,
    plan_kraken_ws_streamers,
};
use typhoon_broker_runtime::kraken_ohlc_pipeline::{
    format_xstock_ws_symbol, kraken_ws_bar_cache_target,
};
pub(super) use typhoon_broker_runtime::kraken_ohlc_pipeline::{
    spawn_kraken_ohlc_pipeline, spawn_kraken_ohlc_snapshot_sweep,
};
#[cfg(test)]
use typhoon_engine::broker::kraken::KrakenWsOhlcBar;
use typhoon_engine::broker::kraken::{
    KRAKEN_WS_OHLC_INTERVALS_MIN, kraken_ws_interval_to_tf_label,
};

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
        if now.duration_since(self.kraken_ws_ohlc_snapshot_sweep_last_schedule)
            < KRAKEN_WS_SNAPSHOT_SWEEP_CADENCE
        {
            return false;
        }
        // Honor the post-failure cooldown: a WS-connect 429 means Kraken is
        // rate-limiting new connections, so don't refire the sweep every cadence
        // slot (that just feeds the limiter and livelocks on the low TFs).
        if let Some(until) = self.kraken_ws_ohlc_snapshot_sweep_backoff_until {
            if now < until {
                return false;
            }
        }
        // Spend this cadence slot now, whether or not there is work, so an
        // all-fresh catalog doesn't re-scan every frame.
        self.kraken_ws_ohlc_snapshot_sweep_last_schedule = now;
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
        // High-timeframe-FIRST coverage: sweep the highest enabled interval that
        // still has MISSING (non-WS-fresh) pairs. W1/D1 finish before the low-TF
        // breadth (1Min/5Min) is touched; already-fresh high TFs fall through, so
        // low TFs still refresh on their short fresh windows once covered. A pair
        // re-arms automatically once its newest bar ages past the WS-fresh window,
        // and a fully-fresh catalog yields no batch (the sweep stays idle).
        let now_ms = chrono::Utc::now().timestamp_millis();
        let Some((interval_min, pairs)) = select_kraken_ws_snapshot_sweep_batch_high_first(
            &catalog,
            &intervals_min,
            &self.kraken_ws_fresh_until,
            &self.kraken_ws_snapshot_attempt,
            now_ms,
            KRAKEN_WS_SNAPSHOT_SWEEP_BATCH_SIZE,
        ) else {
            return false;
        };
        let tf = kraken_ws_interval_to_tf_label(interval_min).unwrap_or("?");
        let pair_count = pairs.len();
        // Record the attempt so a no-data pair backs off instead of being
        // re-selected next cadence; a non-empty commit will set real WS-freshness.
        for ws in &pairs {
            if let Some((_src, symbol)) = kraken_ws_bar_cache_target(ws) {
                self.kraken_ws_snapshot_attempt
                    .insert((symbol, tf.to_string()), now_ms);
            }
        }
        self.kraken_ws_ohlc_snapshot_sweep_in_flight = true;
        let _ = self.broker_tx.send(BrokerCmd::KrakenOhlcSnapshotSweep {
            interval_min,
            pairs,
        });
        self.log.push_back(LogEntry::info(format!(
            "Kraken WS OHLC snapshot sweep: queued {pair_count} missing xStocks for {tf} (high-TF-first)"
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

const KRAKEN_WS_SNAPSHOT_SWEEP_BATCH_SIZE: usize = 250;
/// Smaller per-tick cap for the low timeframes (1Min/5Min), which carry the
/// breadth and the bulk of the snapshot-processing cost. A 150+-pair 1Min gap is
/// spread across sweep ticks (10s apart) instead of landing as one multi-second
/// snapshot burst on the render thread (the overnight 3-5s stalls).
const KRAKEN_WS_SNAPSHOT_SWEEP_LOW_TF_BATCH_SIZE: usize = 32;
const KRAKEN_WS_SNAPSHOT_SWEEP_CADENCE: Duration = Duration::from_secs(10);
/// After a pair is swept, suppress re-selecting it for this long *unless* it goes
/// WS-fresh first. Kraken serves no bars for some `{SYM}x/USD` at some intervals
/// (e.g. weekly on a thinly-traded token); freshness is only marked on a NON-empty
/// commit, so without this backoff those pairs read "missing" forever and — because
/// the sweep is high-timeframe-FIRST — wedge the whole sweep on them every 10s
/// cadence, starving the lower intervals and churning the WS connection. 20 min is
/// shorter than the smallest fresh window (1Min = 24 min), so pairs that DO get
/// data are unaffected; pairs that don't simply retry every 20 min instead of 10s.
const KRAKEN_WS_SNAPSHOT_SWEEP_RETRY_BACKOFF_MS: i64 = 20 * 60 * 1000;

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

/// Pick the snapshot-sweep batch high-timeframe-FIRST: the highest enabled
/// interval that still has missing (non-WS-fresh) xStock pairs, capped at
/// `batch_size`. `None` when every interval is fully fresh.
///
/// This finishes high-TF coverage (W1/D1) before spending sweep ticks on the
/// low-TF breadth (1Min/5Min) that dominates the snapshot cost and produced the
/// multi-second stalls in the overnight log. Once a high TF is fully fresh it
/// falls through to the next; high-TF fresh windows are long (days), so after
/// initial coverage the low TFs — which re-arm on their short fresh windows —
/// get serviced, just at lower priority than any high-TF gap.
fn select_kraken_ws_snapshot_sweep_batch_high_first(
    catalog_symbols: &[String],
    intervals_high_first: &[u32],
    fresh_until: &std::collections::HashMap<(String, String), i64>,
    attempt: &std::collections::HashMap<(String, String), i64>,
    now_ms: i64,
    batch_size: usize,
) -> Option<(u32, Vec<String>)> {
    let batch_size = batch_size.max(1);
    let pairs: Vec<String> = catalog_symbols
        .iter()
        .filter_map(|symbol| format_xstock_ws_symbol(symbol))
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect();
    if pairs.is_empty() {
        return None;
    }
    for &interval_min in intervals_high_first {
        let Some(tf) = kraken_ws_interval_to_tf_label(interval_min) else {
            continue;
        };
        // Low TFs carry the breadth + snapshot cost — cap their per-tick batch
        // smaller so a large 1Min/5Min gap is spread across ticks.
        let cap = if matches!(tf, "1Min" | "5Min") {
            batch_size.min(KRAKEN_WS_SNAPSHOT_SWEEP_LOW_TF_BATCH_SIZE)
        } else {
            batch_size
        };
        let mut missing: Vec<String> = Vec::new();
        for ws in &pairs {
            let is_missing = match kraken_ws_bar_cache_target(ws) {
                Some((_src, symbol)) => {
                    let fresh =
                        TyphooNApp::kraken_ws_pair_is_fresh_at(fresh_until, &symbol, tf, now_ms);
                    // Swept recently but still not fresh → Kraken serves no bars for
                    // this pair/interval. Back off instead of re-arming it every
                    // cadence (which wedges high-TF-first on no-data pairs).
                    let backed_off =
                        attempt
                            .get(&(symbol.clone(), tf.to_string()))
                            .is_some_and(|&t| {
                                now_ms.saturating_sub(t) < KRAKEN_WS_SNAPSHOT_SWEEP_RETRY_BACKOFF_MS
                            });
                    !fresh && !backed_off
                }
                None => true,
            };
            if is_missing {
                missing.push(ws.clone());
                if missing.len() >= cap {
                    break;
                }
            }
        }
        if !missing.is_empty() {
            return Some((interval_min, missing));
        }
    }
    None
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

#[cfg(test)]
mod tests;
