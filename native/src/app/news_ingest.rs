//! Background hydrator for full news article bodies.
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

/// Maximum URLs to fetch per hydration tick. Sized so a typical run takes
/// well under a second on a warm DNS cache and the egui thread never
/// notices.
const HYDRATE_BATCH: usize = 8;

/// Minimum gap between hydration ticks. Body fetches happen in the
/// background — fetching too aggressively burns bandwidth and could be
/// flagged by publishers as scraping.
pub const HYDRATE_INTERVAL_SECS: u64 = 90;

/// Hydrate up to [`HYDRATE_BATCH`] missing-body articles. Returns the
/// number of bodies actually written. Caller is expected to throttle by
/// `HYDRATE_INTERVAL_SECS` between calls; this fn itself does no
/// rate-limiting beyond capping the batch size.
///
/// `symbol_hint`: when `Some`, prefers articles tied to that symbol. The
/// app wires this to the symbol on the currently-visible chart so the
/// user's foreground reading material is hydrated first.
pub async fn hydrate_missing_bodies(
    cache: Arc<SqliteCache>,
    symbol_hint: Option<String>,
) -> usize {
    let targets = match cache.connection() {
        Ok(conn) => news::list_articles_missing_body(
            &conn,
            symbol_hint.as_deref(),
            HYDRATE_BATCH,
        )
        .unwrap_or_default(),
        Err(_) => return 0,
    };
    if targets.is_empty() {
        return 0;
    }

    // Issue all fetches concurrently so per-host RTT doesn't serialise the
    // batch. `HYDRATE_BATCH` is the concurrency cap.
    let fetches = targets
        .into_iter()
        .map(|(url_hash, url)| async move {
            let body = news::fetch_article_body(&url).await;
            (url_hash, body)
        });
    let results: Vec<(String, Option<String>)> =
        futures_util::future::join_all(fetches).await;

    let conn = match cache.connection() {
        Ok(conn) => conn,
        Err(_) => return 0,
    };
    let mut written = 0usize;
    for (url_hash, body) in results {
        let Some(body) = body else { continue };
        if news::upsert_news_body(&conn, &url_hash, &body).is_ok() {
            written += 1;
        }
    }
    written
}
