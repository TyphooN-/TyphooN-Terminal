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
/// Sized against the broker's 500ms inter-symbol pacing: 128 symbols is a
/// ≤64s worst-case run, comfortably inside the default 10-minute interval, so
/// the sweep never overlaps itself or pins `news_loading` (and therefore
/// `heavy_sync_in_progress`) on. Already-fresh symbols skip without network or
/// sleep, so a warm corpus finishes a batch in well under that.
pub(super) const BATCH: usize = 128;

/// How much of each batch is reserved for the active set (watchlist, positions,
/// MTF grid, open charts). The remainder is always available to the rotation
/// cursor, so a user with hundreds of active symbols cannot starve the sweep of
/// the broad universe — it only ever gets the freshest half of its own list.
pub(super) const ACTIVE_SLOTS: usize = BATCH / 2;

/// Cap on the interval accepted from the `NEWSAUTO` command — an hour between
/// batches already means days per sweep of a full universe; beyond that the
/// feature is off in all but name, and `NEWSAUTO OFF` says so honestly.
pub(super) const MAX_INTERVAL_SECS: u64 = 3600;
/// Floor on the same. The broker paces at 500ms/symbol, so a batch cannot
/// complete faster than ~64s; anything under a minute would just queue.
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
        // same provider quota. The existing watchdog in `app_runtime_support`
        // un-latches it if a broker result is ever lost, so this cannot wedge.
        if self.news_loading {
            return;
        }
        // Never add network + SQLite pressure while market-data catch-up is
        // already saturating both. Same rule the manual scope scrape follows.
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
        let count = batch.len();
        let _ = self.broker_tx.send(BrokerCmd::NewsScrapeSymbols {
            symbols: batch,
            marketaux_key: self.marketaux_key.clone(),
            alpha_vantage_key: self.alpha_vantage_key.clone(),
            fmp_key: self.fmp_key.clone(),
            finnhub_key: self.finnhub_key.clone(),
            cryptopanic_key: self.cryptopanic_key.clone(),
        });
        let universe = self.news_auto_scrape_universe.len();
        self.log.push_back(LogEntry::info(format!(
            "News auto-scrape: {count} symbol(s) — cursor {}/{universe}, sweep {}",
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
