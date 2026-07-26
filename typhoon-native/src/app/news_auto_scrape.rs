//! Background news auto-scrape.
//!
//! News was the odd one out. SEC and fundamentals both auto-start at startup
//! (`app_startup.rs`) and retry themselves when a broker universe lands
//! (`start_deferred_scope_scrapes_after_kraken_universe`), so their tables fill
//! on their own. News had every equivalent piece — `news_scrape_scope_symbols`,
//! `BrokerCmd::NewsScrapeSymbols`, a `research_news_scrape_index` freshness
//! table, a full multi-provider fetch — and **no caller**. Every path into it
//! was a button: "Fetch All Sources" (one symbol), "Fetch (All/Active)", the
//! right-panel fetch. So the corpus only ever grew for symbols the user
//! manually pulled, which is exactly the reported "news does not sync unless I
//! force it on select pairs".
//!
//! Why a rotating sweep rather than the SEC one-shot: SEC auto-scrape is
//! deliberately capped at 512 symbols and never repeats, because filings are
//! not time-sensitive within a session. News is the opposite — it is worthless
//! stale and the universe is 10k+ symbols, so a single bounded pass would cover
//! ~4% of it once and never again. A cursor that advances one batch per tick
//! keeps the per-tick cost bounded *and* reaches the whole universe.
//!
//! Cost per frame is the point of the design. The steady state is four bool /
//! integer compares before any allocation:
//!
//! 1. enabled?
//! 2. a scrape already in flight?
//! 3. heavy sync running?
//! 4. has the interval elapsed?
//!
//! Only when (4) passes does anything allocate, and even then the 10k+ scope
//! expansion is cached behind the scope membership signature, so a firing tick
//! is O(batch) and not O(universe). The News window already refuses to expand
//! ALL per frame for this reason; this module holds the same line.

use super::*;

/// Seconds between sweep batches. The broker skips any symbol scraped inside
/// its own 30-minute freshness window (`fresh_news_symbols`), so a tighter
/// interval buys coverage rather than duplicate network — the throttle that
/// matters lives server-side, keyed on real scrape timestamps.
pub(super) const DEFAULT_INTERVAL_SECS: u64 = 600;

/// Symbols dispatched per batch.
///
/// Sized to keep GDELT's real five-second global request budget busy: 64 active
/// slots preserve foreground priority while 128 rotating slots double broad
/// progress from the previous 64-slot rotation. Total runtime also depends on
/// how many active symbols are cold, so the dedicated in-flight latch—not the
/// ten-minute timer—prevents overlap. Already-fresh symbols skip without network
/// or sleep, so warm batches finish much faster.
pub(super) const BATCH: usize = 192;

/// How much of each batch is reserved for the active set (watchlist, positions,
/// MTF grid, open charts). The remainder is always available to the rotation
/// cursor, so a user with hundreds of active symbols cannot starve the sweep of
/// the broad universe. The active reservation remains fixed as broad throughput
/// scales, so foreground news priority is not traded away for catalog coverage.
pub(super) const ACTIVE_SLOTS: usize = 64;

/// Cap on the interval accepted from the `NEWSAUTO` command — an hour between
/// batches already means days per sweep of a full universe; beyond that the
/// feature is off in all but name, and `NEWSAUTO OFF` says so honestly.
pub(super) const MAX_INTERVAL_SECS: u64 = 3600;
/// Floor on the same. The broker and GDELT provider impose their own stricter
/// pacing, and the dedicated in-flight latch prevents interval overlap.
pub(super) const MIN_INTERVAL_SECS: u64 = 60;

impl TyphooNApp {
    /// One tick of the rotating news sweep. Called every `logic()` pass;
    /// returns after a handful of compares unless the interval has elapsed.
    pub(super) fn tick_news_auto_scrape(&mut self, now_instant: std::time::Instant) {
        if !self.news_auto_scrape_enabled {
            return;
        }
        // `news_loading` covers both the manual buttons and our own dispatch,
        // so this is the mutual exclusion that stops two scrapes racing on the
        // same provider quota. The dedicated auto-scrape latch gets a longer
        // watchdog because a cold GDELT-paced batch can legitimately exceed the
        // five-minute manual-fetch timeout.
        if self.news_loading || self.news_auto_scrape_in_flight {
            return;
        }
        // Never add network + SQLite pressure while market-data catch-up is
        // already saturating both. This is the *unattended* sweep, so backing
        // off costs nothing; a manual Fetch click is explicit and runs anyway.
        if self.heavy_sync_in_progress {
            return;
        }
        if !self.cache_loaded {
            return;
        }
        if let Some(last) = self.news_auto_scrape_last_at {
            if now_instant.duration_since(last)
                < std::time::Duration::from_secs(self.news_auto_scrape_interval_secs)
            {
                return;
            }
        }

        // ── Past here the tick actually fires (once per interval) ──
        self.refresh_news_auto_scrape_universe();
        let batch = self.take_news_auto_scrape_batch();
        if batch.is_empty() {
            // Universes still loading. Do not stamp `last_at`: retry on the
            // next tick rather than idling a full interval, mirroring the
            // deferred SEC/fundamentals auto-scrapes.
            return;
        }

        self.news_auto_scrape_last_at = Some(now_instant);
        self.news_loading = true;
        self.news_auto_scrape_in_flight = true;
        self.news_auto_scrape_request_id = self.news_auto_scrape_request_id.wrapping_add(1);
        let request_id = self.news_auto_scrape_request_id;
        let count = batch.len();
        let active = self.active_news_scrape_symbols();
        let keyed_source_symbols = keyed_source_symbols_for_batch(&batch, &active);
        let keyed_count = keyed_source_symbols.len();
        if self
            .broker_tx
            .send(BrokerCmd::NewsScrapeSymbols {
                symbols: batch,
                request_id: Some(request_id),
                keyed_source_symbols,
                marketaux_key: self.marketaux_key.clone(),
                alpha_vantage_key: self.alpha_vantage_key.clone(),
                fmp_key: self.fmp_key.clone(),
                finnhub_key: self.finnhub_key.clone(),
                cryptopanic_key: self.cryptopanic_key.clone(),
            })
            .is_err()
        {
            self.news_loading = false;
            self.news_auto_scrape_in_flight = false;
            self.log.push_back(LogEntry::err(
                "News auto-scrape dispatch failed: broker channel closed".to_string(),
            ));
            return;
        }
        let universe = self.news_auto_scrape_universe.len();
        self.log.push_back(LogEntry::info(format!(
            "News auto-scrape: {count} symbol(s) ({keyed_count} active full-source, {} broad open-source) — cursor {}/{universe}, sweep {}",
            count.saturating_sub(keyed_count),
            self.news_auto_scrape_cursor.min(universe),
            self.news_auto_scrape_sweeps + 1
        )));
    }

    /// Rebuild the cached scope expansion when the scope set moves.
    ///
    /// Keyed on the same membership signature the fundamentals and SEC caches
    /// use, so a broker catalog landing mid-session widens the sweep instead of
    /// leaving it pinned to whatever was loaded at startup — the invalidation
    /// bug that hit Alpaca and then Kraken scope.
    fn refresh_news_auto_scrape_universe(&mut self) {
        let key = super::style_scope::sec_scope_identity_key(
            self.broker_scope,
            self.broker_scope_membership_signature(),
        );
        if self.news_auto_scrape_universe_key == Some(key) {
            return;
        }
        let mut universe = self.news_scrape_scope_symbols();
        universe.sort();
        universe.dedup();
        // A changed universe invalidates the old cursor position: the list is
        // re-sorted, so the index no longer points where it did. Restart the
        // sweep rather than skipping an arbitrary slice.
        self.news_auto_scrape_cursor = 0;
        self.news_auto_scrape_universe = universe;
        self.news_auto_scrape_universe_key = Some(key);
    }

    /// Next batch: active symbols first, then the rotating slice.
    ///
    /// Active symbols (watchlist, positions, MTF grid, open charts) ride along
    /// on *every* batch — they are what the user is looking at, and the
    /// server-side freshness window means re-listing them costs a skip, not a
    /// fetch. The rest of the batch walks the cursor so the broad universe is
    /// covered over successive ticks.
    fn take_news_auto_scrape_batch(&mut self) -> Vec<String> {
        let mut active: Vec<String> = self.active_news_scrape_symbols().into_iter().collect();
        // `active_news_scrape_symbols` is a HashSet: sort before truncating or a
        // large active set contributes an arbitrary subset on every batch.
        active.sort();
        active.truncate(ACTIVE_SLOTS);

        let mut seen: std::collections::HashSet<String> = std::collections::HashSet::new();
        let mut batch: Vec<String> = Vec::with_capacity(BATCH);
        for sym in active {
            if seen.insert(sym.clone()) {
                batch.push(sym);
            }
        }

        // Rotation slots are a fixed share of the batch, not "whatever the
        // active set did not use". Sizing them off `batch.len()` meant a user
        // holding BATCH-or-more active symbols left zero room for the sweep,
        // the cursor never advanced, and the broad universe was never reached —
        // the exact failure this module exists to fix.
        let (start, end, sweeps) = rotation_slice(
            self.news_auto_scrape_cursor,
            self.news_auto_scrape_universe.len(),
            BATCH - ACTIVE_SLOTS,
            self.news_auto_scrape_sweeps,
        );
        self.news_auto_scrape_sweeps = sweeps;
        if start == end {
            batch.sort();
            return batch;
        }
        for sym in &self.news_auto_scrape_universe[start..end] {
            if seen.insert(sym.clone()) {
                batch.push(sym.clone());
            }
        }
        // Advance by the slice consumed, not by what landed in the batch:
        // symbols dropped as duplicates of the active set were still covered,
        // so counting them as un-swept would stall the cursor.
        self.news_auto_scrape_cursor = end;
        batch.sort();
        batch
    }
}

/// Keep scarce keyed-provider quotas on symbols the user is actively tracking.
/// Broad rotation symbols still use GDELT, Yahoo RSS, and CoinDesk; including
/// keyed providers for a 10k+ catalog cannot converge within their daily limits
/// and only starves foreground fetches.
pub(super) fn keyed_source_symbols_for_batch(
    batch: &[String],
    active: &std::collections::HashSet<String>,
) -> Vec<String> {
    let active: std::collections::HashSet<String> = active
        .iter()
        .map(|symbol| {
            normalize_market_data_symbol(symbol)
                .replace('/', "")
                .to_uppercase()
        })
        .collect();
    batch
        .iter()
        .filter(|symbol| {
            active.contains(
                &normalize_market_data_symbol(symbol)
                    .replace('/', "")
                    .to_uppercase(),
            )
        })
        .cloned()
        .collect()
}

/// Parse the argument of the `NEWSAUTO` console command.
///
/// `ON` / `OFF` toggle the sweep; a bare number sets the batch interval in
/// minutes (clamped to [`MIN_INTERVAL_SECS`], [`MAX_INTERVAL_SECS`]). An empty
/// argument reports current state without changing it.
pub(super) fn parse_news_auto_command(arg: &str) -> NewsAutoCommand {
    let arg = arg.trim();
    if arg.is_empty() {
        return NewsAutoCommand::Report;
    }
    match arg.to_ascii_uppercase().as_str() {
        "ON" | "ENABLE" | "ENABLED" => NewsAutoCommand::Enable,
        "OFF" | "DISABLE" | "DISABLED" => NewsAutoCommand::Disable,
        other => match other.trim_end_matches("M").parse::<u64>() {
            Ok(mins) if mins > 0 => NewsAutoCommand::Interval(
                (mins.saturating_mul(60)).clamp(MIN_INTERVAL_SECS, MAX_INTERVAL_SECS),
            ),
            _ => NewsAutoCommand::Invalid,
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum NewsAutoCommand {
    Report,
    Enable,
    Disable,
    Interval(u64),
    Invalid,
}

/// Pure rotation step, extracted so the cursor arithmetic is testable without
/// a `TyphooNApp`. Returns the slice bounds to take and the post-batch cursor /
/// sweep count.
///
/// `slots` is the room left for the rotation after the active reservation —
/// deliberately a constant share of the batch rather than "whatever the active
/// set did not use", so the cursor always advances.
pub(super) fn rotation_slice(
    cursor: usize,
    universe_len: usize,
    slots: usize,
    sweeps: u64,
) -> (usize, usize, u64) {
    if universe_len == 0 || slots == 0 {
        return (cursor, cursor, sweeps);
    }
    let (start, sweeps) = if cursor >= universe_len {
        (0, sweeps.wrapping_add(1))
    } else {
        (cursor, sweeps)
    };
    (start, (start + slots).min(universe_len), sweeps)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotation_covers_the_whole_universe_and_wraps() {
        // A sweep must reach every symbol, not re-scrape the head forever —
        // that is the entire difference between this and the SEC one-shot.
        let universe_len = 10usize;
        let slots = 4usize;
        let mut cursor = 0usize;
        let mut sweeps = 0u64;
        let mut covered: Vec<usize> = Vec::new();
        for _ in 0..3 {
            let (start, end, next_sweeps) = rotation_slice(cursor, universe_len, slots, sweeps);
            covered.extend(start..end);
            cursor = end;
            sweeps = next_sweeps;
        }
        assert_eq!(covered, (0..universe_len).collect::<Vec<_>>());
        assert_eq!(sweeps, 0, "a sweep is only counted once the cursor wraps");

        // Next tick wraps and starts the second pass.
        let (start, end, sweeps) = rotation_slice(cursor, universe_len, slots, sweeps);
        assert_eq!((start, end), (0, 4));
        assert_eq!(sweeps, 1);
    }

    #[test]
    fn rotation_always_advances_so_active_symbols_cannot_starve_the_sweep() {
        // Regression: sizing the rotation slice off the *remaining* batch space
        // meant a user with BATCH-or-more active symbols left zero room, the
        // cursor never moved, and the broad universe was never scraped.
        assert!(
            ACTIVE_SLOTS < BATCH,
            "the rotation must always keep a reserved share of the batch"
        );
        let slots = BATCH - ACTIVE_SLOTS;
        assert!(slots > 0);
        let (start, end, _) = rotation_slice(0, 10_000, slots, 0);
        assert_eq!(end - start, slots, "every tick must consume rotation slots");
    }

    #[test]
    fn default_batch_doubles_rotation_without_reducing_active_priority() {
        // Keep the same 64-symbol foreground reservation while doubling the
        // rotating share from the previous 64 to 128. Provider pacing controls
        // elapsed time; the in-flight latch prevents a timer-driven overlap.
        assert_eq!(ACTIVE_SLOTS, 64);
        assert_eq!(BATCH - ACTIVE_SLOTS, 128);
    }

    #[test]
    fn keyed_provider_quota_is_reserved_for_active_symbols() {
        let batch = vec![
            "AAPL.EQ".into(),
            "MSFT".into(),
            "ETHUSD".into(),
            "ZZZZ".into(),
        ];
        let active =
            std::collections::HashSet::from(["AAPL".into(), "MSFT".into(), "ETH/USD".into()]);

        assert_eq!(
            keyed_source_symbols_for_batch(&batch, &active),
            ["AAPL.EQ", "MSFT", "ETHUSD"]
        );
    }

    #[test]
    fn rotation_is_a_noop_before_the_universe_loads() {
        // Universes arrive asynchronously; an empty list must not advance the
        // cursor or bump the sweep counter (the caller retries next tick).
        assert_eq!(rotation_slice(0, 0, 64, 7), (0, 0, 7));
    }

    #[test]
    fn rotation_final_slice_is_clamped_to_the_universe() {
        // Last batch of a pass is short rather than out of bounds.
        let (start, end, sweeps) = rotation_slice(8, 10, 64, 0);
        assert_eq!((start, end), (8, 10));
        assert_eq!(sweeps, 0);
    }

    #[test]
    fn news_auto_command_parses_toggles_and_intervals() {
        assert_eq!(parse_news_auto_command(""), NewsAutoCommand::Report);
        assert_eq!(parse_news_auto_command("  "), NewsAutoCommand::Report);
        assert_eq!(parse_news_auto_command("on"), NewsAutoCommand::Enable);
        assert_eq!(parse_news_auto_command("OFF"), NewsAutoCommand::Disable);
        assert_eq!(
            parse_news_auto_command("15"),
            NewsAutoCommand::Interval(900)
        );
        // Minutes suffix is accepted so `NEWSAUTO 15m` does not silently fail.
        assert_eq!(
            parse_news_auto_command("15m"),
            NewsAutoCommand::Interval(900)
        );
        // Clamped at both ends rather than rejected — an out-of-range number is
        // a clear intent ("as fast/slow as you can"), not a typo.
        assert_eq!(
            parse_news_auto_command("1"),
            NewsAutoCommand::Interval(MIN_INTERVAL_SECS)
        );
        assert_eq!(
            parse_news_auto_command("9999"),
            NewsAutoCommand::Interval(MAX_INTERVAL_SECS)
        );
        assert_eq!(parse_news_auto_command("0"), NewsAutoCommand::Invalid);
        assert_eq!(parse_news_auto_command("later"), NewsAutoCommand::Invalid);
    }
}
