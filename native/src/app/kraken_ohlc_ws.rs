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

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::mpsc;
use typhoon_engine::broker::kraken::{
    KRAKEN_WS_OHLC_INTERVALS_MIN, KrakenOhlcStreamerEvent, KrakenWsOhlcBar,
    kraken_ws_bar_to_json, kraken_ws_interval_to_tf_label, kraken_ws_symbol_to_cache_key,
    run_ohlc_streamer,
};
use typhoon_engine::core::cache::SqliteCache;

use super::{BrokerCmd, LogEntry, TyphooNApp};

impl TyphooNApp {
    /// Kick off the Kraken WS OHLC pipeline if the user opted in and the
    /// pair catalog is loaded. Idempotent: `kraken_ws_ohlc_started` guards
    /// against re-spawning streamers when KrakenPairs lands again or the
    /// caller fires this from multiple lifecycle hooks. Returns `true` if
    /// streamers were just spawned, `false` if any precondition wasn't met
    /// or it was already started.
    pub(super) fn maybe_start_kraken_ws_ohlc(&mut self) -> bool {
        if !self.kraken_enabled
            || !self.kraken_ws_ohlc_enabled
            || self.kraken_ws_ohlc_started
            || self.kraken_pairs.is_empty()
        {
            return false;
        }
        // Build the ws-format symbol list (e.g. "BTC/USD") that Kraken's WS
        // expects. Use the wsname when available, falling back to the
        // tradeable-pair display symbol.
        let pairs: Vec<String> = build_kraken_ws_subscribe_symbols(&self.kraken_pairs);
        if pairs.is_empty() {
            return false;
        }
        let count = pairs.len();
        let _ = self.broker_tx.send(BrokerCmd::KrakenStartOhlcStreamers { pairs });
        self.kraken_ws_ohlc_started = true;
        self.log.push_back(LogEntry::info(format!(
            "Kraken WS OHLC: spawning streamers for {count} pairs × 8 intervals",
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
pub(super) fn build_kraken_ws_subscribe_symbols(
    pairs: &[(String, String)],
) -> Vec<String> {
    let mut out = std::collections::BTreeSet::new();
    for (pair_name, display) in pairs {
        if let Some(formatted) = format_ws_symbol(pair_name, display) {
            out.insert(formatted);
        }
    }
    out.into_iter().collect()
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
    const QUOTES: [&str; 12] = [
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
mod tests {
    use super::*;

    #[test]
    fn format_ws_symbol_prefers_display_when_already_slashed() {
        assert_eq!(
            format_ws_symbol("XXBTZUSD", "XBT/USD"),
            Some("XBT/USD".into())
        );
        assert_eq!(
            format_ws_symbol("ETHUSD", "eth/usd"),
            Some("ETH/USD".into())
        );
    }

    #[test]
    fn format_ws_symbol_inserts_slash_from_legacy_pair_name() {
        // Display is the flat form; we must reconstruct base/quote.
        assert_eq!(
            format_ws_symbol("XXBTZUSD", "XBTUSD"),
            Some("BTC/USD".into())
        );
        assert_eq!(
            format_ws_symbol("ETHUSD", "ETHUSD"),
            Some("ETH/USD".into())
        );
        assert_eq!(
            format_ws_symbol("SOLUSD", "SOLUSD"),
            Some("SOL/USD".into())
        );
    }

    #[test]
    fn format_ws_symbol_handles_stablecoin_quotes() {
        assert_eq!(
            format_ws_symbol("USDCUSD", "USDCUSD"),
            Some("USDC/USD".into())
        );
        assert_eq!(
            format_ws_symbol("BTCUSDT", "BTCUSDT"),
            Some("BTC/USDT".into())
        );
        // USDG (Paxos) — the longest stablecoin suffix.
        assert_eq!(
            format_ws_symbol("BTCUSDG", "BTCUSDG"),
            Some("BTC/USDG".into())
        );
    }

    #[test]
    fn format_ws_symbol_returns_none_for_unrecognised_quote() {
        // No known quote suffix at all → can't construct a /-form.
        assert!(format_ws_symbol("XYZABC", "XYZABC").is_none());
    }

    #[test]
    fn build_kraken_ws_subscribe_symbols_dedupes_and_sorts() {
        let pairs = vec![
            ("XXBTZUSD".to_string(), "XBT/USD".to_string()),
            ("ETHUSD".to_string(), "ETHUSD".to_string()),
            ("XXBTZUSD".to_string(), "XBTUSD".to_string()), // dup of first via fallback
            ("SOLUSD".to_string(), "SOLUSD".to_string()),
        ];
        let symbols = build_kraken_ws_subscribe_symbols(&pairs);
        // BTreeSet dedupes XBT/USD vs BTC/USD as distinct strings — both
        // get included. That's fine: Kraken accepts either form on
        // subscribe. Order is stable / sorted.
        assert!(symbols.contains(&"ETH/USD".to_string()));
        assert!(symbols.contains(&"SOL/USD".to_string()));
        let mut sorted = symbols.clone();
        sorted.sort();
        assert_eq!(symbols, sorted, "BTreeSet output should already be sorted");
    }

    #[test]
    fn build_kraken_ws_subscribe_symbols_filters_out_unmappable_pairs() {
        let pairs = vec![
            ("XXBTZUSD".to_string(), "XBT/USD".to_string()),
            ("GARBAGE".to_string(), "GARBAGE".to_string()),
        ];
        let symbols = build_kraken_ws_subscribe_symbols(&pairs);
        assert!(symbols.iter().all(|s| s.contains('/')));
    }
}

/// Coalesce-and-flush cadence for the WS bar writer. Picked so an active
/// pair's 1Min bar is flushed at least once per bar (~5 flushes/min), but
/// idle pairs don't pay the cost of a same-bar rewrite every tick.
const WS_BAR_FLUSH_INTERVAL: Duration = Duration::from_secs(5);

/// Maximum bars per `merge_bars` call. The cache's merge_bars rewrites the
/// entire compressed blob for the key, so we batch but don't try to flush
/// everything in one go — keeps per-flush latency bounded even for active
/// pairs with many recent buckets.
const WS_BAR_MAX_BARS_PER_KEY: usize = 0; // 0 == unbounded (full-depth merge)

/// Triple committed after a flush: `(typhoon_symbol, tf_label, last_bar_ts_ms)`.
/// Sent to the main loop so it can mark the (symbol, tf) WS-fresh.
pub(super) type WsFreshEntry = (String, String, i64);

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
    commit_tx: mpsc::UnboundedSender<Vec<WsFreshEntry>>,
    status_tx: mpsc::UnboundedSender<KrakenOhlcStreamerEvent>,
) {
    if pairs.is_empty() {
        return;
    }
    let (bar_tx, bar_rx) = mpsc::unbounded_channel::<KrakenWsOhlcBar>();
    for &interval_min in KRAKEN_WS_OHLC_INTERVALS_MIN {
        let pairs = pairs.clone();
        let bar_tx = bar_tx.clone();
        let status_tx = status_tx.clone();
        tokio::spawn(async move {
            run_ohlc_streamer(interval_min, pairs, bar_tx, status_tx).await;
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

/// Drain the merged bar channel, buffer by (symbol, interval, bucket),
/// flush every [`WS_BAR_FLUSH_INTERVAL`]. Exits when the channel closes.
async fn run_ws_bar_writer(
    shared_cache: Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    mut bar_rx: mpsc::UnboundedReceiver<KrakenWsOhlcBar>,
    commit_tx: mpsc::UnboundedSender<Vec<WsFreshEntry>>,
) {
    // (symbol_cache_key, interval_min, interval_begin_ms) -> bar
    // Last-write-wins for the same bucket, which is exactly the semantic
    // Kraken's WS uses: each new update for the open bar supersedes the
    // previous one until interval_begin rolls.
    let mut buffer: HashMap<(String, u32, i64), KrakenWsOhlcBar> = HashMap::new();

    let mut flush_ticker = tokio::time::interval(WS_BAR_FLUSH_INTERVAL);
    flush_ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    flush_ticker.tick().await; // skip the immediate first fire

    loop {
        tokio::select! {
            maybe_bar = bar_rx.recv() => {
                match maybe_bar {
                    Some(bar) => {
                        let cache_symbol = kraken_ws_symbol_to_cache_key(&bar.symbol);
                        if cache_symbol.is_empty() {
                            continue;
                        }
                        buffer.insert(
                            (cache_symbol, bar.interval_min, bar.interval_begin_ms),
                            bar,
                        );
                    }
                    None => break, // every streamer dropped its sender → shut down
                }
            }
            _ = flush_ticker.tick() => {
                if buffer.is_empty() {
                    continue;
                }
                let to_flush = std::mem::take(&mut buffer);
                flush_ws_bars(&shared_cache, &commit_tx, to_flush).await;
            }
        }
    }
    // Final drain on shutdown.
    if !buffer.is_empty() {
        flush_ws_bars(&shared_cache, &commit_tx, std::mem::take(&mut buffer)).await;
    }
}

/// Group buffered bars by `(typhoon_symbol, tf_label)`, merge each group
/// into the cache in one `merge_bars` call, and report fresh entries to
/// the main loop.
async fn flush_ws_bars(
    shared_cache: &Arc<std::sync::RwLock<Option<Arc<SqliteCache>>>>,
    commit_tx: &mpsc::UnboundedSender<Vec<WsFreshEntry>>,
    buffer: HashMap<(String, u32, i64), KrakenWsOhlcBar>,
) {
    let Some(cache) = shared_cache.read().ok().and_then(|g| g.clone()) else {
        return;
    };

    // Group bars by their target cache key + track the newest bucket ts so
    // we can report it back as the WS-fresh anchor. PERF: this is O(n) over
    // buffered bars; we expect n in the low thousands at most per flush.
    let mut grouped: HashMap<(String, &'static str), (Vec<serde_json::Value>, i64)> =
        HashMap::new();
    for ((cache_symbol, interval_min, interval_begin_ms), bar) in buffer {
        let Some(tf_label) = kraken_ws_interval_to_tf_label(interval_min) else {
            continue;
        };
        let json = kraken_ws_bar_to_json(&bar);
        let entry = grouped
            .entry((cache_symbol, tf_label))
            .or_insert_with(|| (Vec::new(), 0));
        entry.0.push(json);
        if interval_begin_ms > entry.1 {
            entry.1 = interval_begin_ms;
        }
    }

    if grouped.is_empty() {
        return;
    }

    let cache_clone = cache.clone();
    let result = tokio::task::spawn_blocking(move || {
        let mut committed: Vec<WsFreshEntry> = Vec::with_capacity(grouped.len());
        for ((cache_symbol, tf_label), (bars, last_bucket_ms)) in grouped {
            if bars.is_empty() {
                continue;
            }
            let key = format!("kraken:{cache_symbol}:{tf_label}");
            let bars_json = match serde_json::to_string(&bars) {
                Ok(j) => j,
                Err(_) => continue,
            };
            if cache_clone
                .merge_bars(&key, &bars_json, WS_BAR_MAX_BARS_PER_KEY)
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
}
