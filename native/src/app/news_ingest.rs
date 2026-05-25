//! Background hydrator for full news article bodies.
//! On first ingest we now attempt to store the full body (not just summary)
//! and deduplicate by URL hash across symbols.
//!
//! `engine::core::news` stores summaries from each provider's news API (~150
//! chars). The full article body is a separate fetch. This module owns the
//! lazy hydration: pick a small batch of cached articles whose `body` is
//! still empty, fetch each, and write the extracted text back to the
//! `research_news.body` column + FTS5 mirror.
//!
//! Pacing is conservative so the hydrator never competes with broker sync
//! or the egui frame:
//!  * One batch per tick, run from the main loop at a slow cadence.
//!  * Bounded concurrency (small `futures::join_all` window).
//!  * Per-host backoff is implicit — we read whatever URLs were cached, so
//!    a single source clustering doesn't hammer one host all at once.
//!
//! Failures are silent: the row stays at `body = ''` and the next pass
//! tries again. There is no retry-limit yet because article URLs are
//! generally stable; if a publisher 4xx's permanently the row just stays
//! at summary-only forever, which is the previous behaviour.
//!
//! ADR-212 (AI return-path auto-ingest) covers the prompt-side use of the
//! body text. ADR-214 covers the broader UI-responsiveness work this
//! lazy-fetch policy is part of.

use std::sync::Arc;

use typhoon_engine::core::cache::SqliteCache;
use typhoon_engine::core::news;

/// Maximum URLs to fetch per hydration tick. Fetches run with the batch
/// as the concurrency cap (`futures::join_all`), so this also bounds the
/// per-tick burst against any single host. URLs in a batch typically span
/// many publishers (yahoo, motleyfool, tickerreport, etc.) so a single
/// burst rarely hits one host more than once or twice.
const HYDRATE_BATCH: usize = 32;

/// Minimum gap between hydration ticks. Combined with [`HYDRATE_BATCH`]
/// this caps the steady-state rate at ~2 fetches/sec across all
/// publishers, well under any reasonable per-host limit.
pub const HYDRATE_INTERVAL_SECS: u64 = 15;

/// Hydrate up to [`HYDRATE_BATCH`] missing-body articles. Returns the
/// number of bodies actually written. Caller is expected to throttle by
/// `HYDRATE_INTERVAL_SECS` between calls; this fn itself does no
/// rate-limiting beyond capping the batch size.
///
/// `symbol_hint`: when `Some`, prefers articles tied to that symbol. The
/// app wires this to the symbol on the currently-visible chart so the
/// user's foreground reading material is hydrated first.
pub async fn hydrate_missing_bodies(cache: Arc<SqliteCache>, symbol_hint: Option<String>) -> usize {
    let targets = match cache.connection() {
        Ok(conn) => news::list_articles_missing_body(&conn, symbol_hint.as_deref(), HYDRATE_BATCH)
            .unwrap_or_default(),
        Err(_) => return 0,
    };
    if targets.is_empty() {
        return 0;
    }

    // Issue all fetches concurrently so per-host RTT doesn't serialise the
    // batch. `HYDRATE_BATCH` is the concurrency cap. We grab the og:image
    // alongside the body so Yahoo (which leaves image_url empty in its
    // RSS) gets a hero image backfilled on first hydration.
    let fetches = targets.into_iter().map(|(url_hash, url)| async move {
        let result = news::fetch_article_body_with_image(&url).await;
        (url_hash, result)
    });
    let results: Vec<(String, Option<(String, String)>)> =
        futures_util::future::join_all(fetches).await;

    let conn = match cache.connection() {
        Ok(conn) => conn,
        Err(_) => return 0,
    };
    let mut written = 0usize;
    for (url_hash, result) in results {
        match result {
            Some((body, image_url)) => {
                if news::upsert_news_body_and_image(&conn, &url_hash, &body, &image_url).is_ok() {
                    written += 1;
                }
            }
            None => {
                // Bump the failure counter so we eventually stop retrying
                // and the UI swaps the "still hydrating" placeholder for a
                // terminal "body unavailable" message.
                let _ = news::bump_news_body_fetch_attempts(&conn, &url_hash);
            }
        }
    }
    written
}

/// Fetch the body for a single URL and write it to the cache. Used by the
/// on-click path so a user clicking an unhydrated article gets immediate
/// feedback instead of waiting for the next background tick. Returns true
/// if a body was actually stored, false if the fetch failed (in which
/// case the per-URL failure counter is bumped, same as the batch path).
pub async fn hydrate_one_url(
    cache: Arc<SqliteCache>,
    url_hash: String,
    url: String,
) -> bool {
    if url.is_empty() {
        return false;
    }
    let result = news::fetch_article_body_with_image(&url).await;
    let Ok(conn) = cache.connection() else {
        return false;
    };
    match result {
        Some((body, image_url)) => {
            news::upsert_news_body_and_image(&conn, &url_hash, &body, &image_url).is_ok()
        }
        None => {
            let _ = news::bump_news_body_fetch_attempts(&conn, &url_hash);
            false
        }
    }
}
