//! News ingest — multi-source aggregation with SQLite cache and FTS5 search.
//!
//! Sources (all free tier, most without API keys):
//!
//! **Equities & general:**
//! - **GDELT 2.0 Doc API** — no key, global coverage, JSON (`https://api.gdeltproject.org/api/v2/doc/doc`)
//! - **Yahoo Finance RSS** — no key, per-symbol (`https://feeds.finance.yahoo.com/rss/2.0/headline?s=SYM`)
//! - **Marketaux** — 100 req/day free, finance-focused, sentiment tags
//! - **Alpha Vantage NEWS_SENTIMENT** — 25 req/day free, built-in sentiment + tickers
//! - **FMP /v3/stock_news** — 250 req/day free, clean normalized format
//! - **Finnhub /company-news** — 60 req/min free, per-symbol equities
//!
//! **Crypto-native:**
//! - **CryptoPanic** — free public tier, per-currency filtering
//! - **CoinDesk RSS** — no key, general feed filtered by base ticker mention
//! - **Finnhub /news?category=crypto** — free with same key as equities, general crypto feed filtered by base
//!
//! `fetch_all_sources_for_symbol` auto-routes between the equity and crypto sets
//! using [`is_crypto_symbol`].
//!
//! All fetchers normalize into `NewsArticle` and upsert into `research_news` keyed by
//! SHA-256 of the canonical URL so the same story from two sources collapses to one row.
//!
//! The `research_news_fts` FTS5 virtual table mirrors headline + summary so the NEWS
//! window can do keyword search across cached articles instantly.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicI64, Ordering};

// ── Rate-limit cooldown ───────────────────────────────────────────────────
//
// GDELT's free Doc API throttles aggressively and returns 429 with no
// Retry-After header. Hitting it once during a bulk per-symbol scrape means
// every subsequent ticker in the loop will also 429, spamming the log. When

use std::time::{SystemTime, UNIX_EPOCH};

mod article_body;
mod crypto_sources;
mod source_fetchers;
pub use article_body::{
    clean_article_body, extract_article_text, extract_article_with_image, fetch_article_body,
    fetch_article_body_with_image,
};
#[cfg(test)]
use crypto_sources::article_mentions_crypto;
use crypto_sources::crypto_full_name;
pub use crypto_sources::{
    crypto_base_for_symbol, fetch_coindesk_rss, fetch_cryptopanic_news, fetch_finnhub_crypto_news,
    is_crypto_symbol,
};
pub use source_fetchers::{
    fetch_alpha_vantage_news, fetch_fmp_news, fetch_gdelt_news, fetch_marketaux_news,
    fetch_sec_edgar_rss, fetch_yahoo_rss, parse_iso_ts,
};
#[cfg(test)]
use source_fetchers::{parse_atom_items, parse_av_ts, parse_gdelt_ts, parse_rss_items, strip_html};

const RATE_LIMIT_COOLDOWN_SECS: i64 = 600;
pub(super) const GDELT_MIN_INTERVAL_SECS: i64 = 5; // Enforce at least 5 seconds between GDELT requests
static GDELT_COOLDOWN_UNTIL: AtomicI64 = AtomicI64::new(0);
pub(super) static GDELT_LAST_REQUEST_TIME: AtomicI64 = AtomicI64::new(0); // Timestamp of the last GDELT request

pub(super) fn now_secs() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_default()
}

/// Seconds remaining on the GDELT 429 cooldown, or 0 if not throttled.
pub fn gdelt_cooldown_remaining_secs() -> i64 {
    (GDELT_COOLDOWN_UNTIL.load(Ordering::Relaxed) - now_secs()).max(0)
}

/// True while GDELT requests should be skipped after a 429.

/// Returns true if the article is relevant enough for the given ticker.
/// For short tickers (≤ 4 chars) we are stricter to avoid noise like the WOK cooking spam.
pub fn article_is_relevant_for_ticker(article: &NewsArticle, ticker: &str) -> bool {
    let t = ticker.to_uppercase();
    if t.len() > 4 {
        return true; // long tickers are usually unique enough
    }

    let text = format!("{} {}", article.headline, article.summary).to_lowercase();
    let company_variants = ["work medical", "workmedical", "wok medical", "wokmedical"];

    // Strong financial / company signals
    let financial_signals = [
        "earnings",
        "revenue",
        "fda",
        "nasdaq",
        "medical device",
        "biopharma",
        "clinical",
        "conference",
        "partnership",
        "subsidiary",
        "acquisition",
    ];

    let has_company = company_variants.iter().any(|v| text.contains(v));
    let has_financial = financial_signals.iter().any(|v| text.contains(v));

    // Accept if it mentions the company name or has clear financial context
    has_company || has_financial
}

pub fn gdelt_in_cooldown() -> bool {
    gdelt_cooldown_remaining_secs() > 0
}

pub(super) fn trip_gdelt_cooldown() {
    GDELT_COOLDOWN_UNTIL.store(now_secs() + RATE_LIMIT_COOLDOWN_SECS, Ordering::Relaxed);
}

// ── Data Type ─────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct NewsArticle {
    pub url_hash: String, // SHA-256 hex of the normalized URL (PK)
    pub symbol: String,   // primary symbol this article is associated with (may be empty)
    pub source: String, // "GDELT" | "YahooRSS" | "Marketaux" | "AlphaVantage" | "FMP" | "Finnhub" | "Alpaca"
    pub provider: String, // original publisher name (e.g. "Reuters", "Bloomberg")
    pub headline: String,
    pub summary: String,
    pub url: String,
    pub published_at: i64, // unix seconds
    pub image_url: String,
    pub sentiment: String, // "bullish" | "bearish" | "neutral" | "" (free text from API if any)
    pub sentiment_score: f64, // -1.0 to 1.0 if API provides, else 0.0
    pub tickers: Vec<String>, // cross-referenced tickers
    pub categories: Vec<String>, // topic tags
    /// Full article body extracted from the source URL. Empty until a fetcher
    /// hydrates it (see [`fetch_article_body`] + [`upsert_news_body`]).
    /// Populated lazily by the native app so the publisher's article text is
    /// in the local SQLite cache for offline reading, AI prompts, and FTS5
    /// search beyond what the upstream news APIs return as `summary`.
    #[serde(default)]
    pub body: String,
    /// Number of times the lazy hydrator tried to fetch the article body and
    /// failed (non-2xx, paywall splash under MIN_BODY_CHARS, timeout, parser
    /// returned empty). Used by the UI to swap the "still hydrating"
    /// placeholder for "body unavailable" once retries are exhausted, and by
    /// `list_articles_missing_body` to skip permanently-broken URLs.
    #[serde(default)]
    pub body_fetch_attempts: i64,
}

/// Hydrator retry budget. After this many failed body fetches the row is
/// considered permanently unhydratable — the UI shows a terminal message and
/// the hydrator stops re-queueing it. Five attempts ≈ ~7.5 minutes of retry
/// at the default 90s tick before giving up, which is plenty for a transient
/// publisher outage but cheap to abandon for the long tail of JS-only pages.
pub const MAX_BODY_FETCH_ATTEMPTS: i64 = 5;

impl NewsArticle {
    fn compute_hash(url: &str) -> String {
        let mut h = Sha256::new();
        h.update(url.trim().to_lowercase().as_bytes());
        let digest = h.finalize();
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }

    pub fn with_hash(mut self) -> Self {
        if self.url_hash.is_empty() && !self.url.is_empty() {
            self.url_hash = Self::compute_hash(&self.url);
        }
        self
    }
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

// ── SQLite schema ─────────────────────────────────────────────────────────

/// Create `research_news` table + FTS5 index + per-symbol recency index.
/// Idempotent — safe to call on every connection open.
pub fn create_news_tables(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_news (
            url_hash TEXT PRIMARY KEY,
            symbol TEXT NOT NULL DEFAULT '',
            source TEXT NOT NULL DEFAULT '',
            provider TEXT NOT NULL DEFAULT '',
            headline TEXT NOT NULL DEFAULT '',
            summary TEXT NOT NULL DEFAULT '',
            url TEXT NOT NULL DEFAULT '',
            published_at INTEGER NOT NULL DEFAULT 0,
            image_url TEXT NOT NULL DEFAULT '',
            sentiment TEXT NOT NULL DEFAULT '',
            sentiment_score REAL NOT NULL DEFAULT 0.0,
            tickers_json TEXT NOT NULL DEFAULT '[]',
            categories_json TEXT NOT NULL DEFAULT '[]',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_news_sym_ts
            ON research_news(symbol, published_at DESC);
        CREATE INDEX IF NOT EXISTS idx_research_news_ts
            ON research_news(published_at DESC);
        CREATE INDEX IF NOT EXISTS idx_research_news_updated
            ON research_news(updated_at);
        CREATE INDEX IF NOT EXISTS idx_research_news_source_sym_ts
            ON research_news(source, symbol, published_at DESC);
        CREATE TABLE IF NOT EXISTS research_news_scrape_index (
            symbol TEXT PRIMARY KEY,
            last_scrape_at INTEGER NOT NULL DEFAULT 0,
            article_count INTEGER NOT NULL DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS research_news_ignored (
            url_hash TEXT PRIMARY KEY,
            symbol TEXT NOT NULL DEFAULT '',
            ignored_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE VIRTUAL TABLE IF NOT EXISTS research_news_fts USING fts5(
            url_hash UNINDEXED,
            headline,
            summary,
            body,
            tokenize='porter unicode61'
        );",
    )
    .map_err(|e| format!("create news tables: {e}"))?;

    // Idempotent migrations for installs that pre-date the full-body column.
    // `ALTER TABLE ADD COLUMN` errors on duplicate; the `_ =` ignores that.
    let _ = conn.execute(
        "ALTER TABLE research_news ADD COLUMN body TEXT NOT NULL DEFAULT ''",
        [],
    );
    let _ = conn.execute(
        "ALTER TABLE research_news ADD COLUMN body_fetched_at INTEGER NOT NULL DEFAULT 0",
        [],
    );
    // body_fetch_attempts: count of times the hydrator tried and failed to
    // extract a usable article body for this URL. After MAX_BODY_FETCH_ATTEMPTS
    // the UI shows "Body unavailable" instead of the misleading "still
    // hydrating" placeholder. Persistent so a re-launch doesn't reset retry
    // budgets.
    let _ = conn.execute(
        "ALTER TABLE research_news ADD COLUMN body_fetch_attempts INTEGER NOT NULL DEFAULT 0",
        [],
    );
    // FTS5 layout cannot ALTER. If an older DB has a 3-column FTS table, drop
    // and rebuild so search hits the body too. Cheap (no UNINDEXED on body
    // means re-tokenising only the rows we've already fetched body for, but
    // those are the only ones with content beyond headline+summary anyway).
    let fts_has_body: bool = conn
        .query_row(
            "SELECT 1 FROM pragma_table_info('research_news_fts') WHERE name='body'",
            [],
            |_| Ok(true),
        )
        .unwrap_or(false);
    if !fts_has_body {
        let _ = conn.execute_batch(
            "DROP TABLE IF EXISTS research_news_fts;
             CREATE VIRTUAL TABLE research_news_fts USING fts5(
                 url_hash UNINDEXED,
                 headline,
                 summary,
                 body,
                 tokenize='porter unicode61'
             );",
        );
        // Repopulate from the main table so search keeps working for already-
        // cached rows without waiting for the next upsert.
        let _ = conn.execute(
            "INSERT INTO research_news_fts(url_hash, headline, summary, body)
             SELECT url_hash, headline, summary, body FROM research_news",
            [],
        );
    }
    Ok(())
}

// ── Upsert / query ────────────────────────────────────────────────────────

/// Insert or update a news article. Dedup is by `url_hash`.
pub fn upsert_news(conn: &Connection, article: &NewsArticle) -> Result<(), String> {
    let _ = create_news_tables(conn);
    let a = if article.url_hash.is_empty() {
        article.clone().with_hash()
    } else {
        article.clone()
    };
    if a.url_hash.is_empty() {
        return Err("url_hash empty after hash".into());
    }

    // Honour the user's "Remove / Ignore" action: never resurrect an article the
    // user explicitly purged (GDELT false positives like the WOK cooking spam).
    if conn
        .query_row(
            "SELECT 1 FROM research_news_ignored WHERE url_hash = ?1",
            params![a.url_hash],
            |_| Ok(()),
        )
        .is_ok()
    {
        return Ok(());
    }

    let tickers_json = serde_json::to_string(&a.tickers).unwrap_or("[]".into());
    let categories_json = serde_json::to_string(&a.categories).unwrap_or("[]".into());

    conn.execute(
        "INSERT INTO research_news
         (url_hash, symbol, source, provider, headline, summary, url, published_at,
          image_url, sentiment, sentiment_score, tickers_json, categories_json, updated_at, body)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15)
         ON CONFLICT(url_hash) DO UPDATE SET
            symbol = CASE WHEN research_news.symbol = '' THEN excluded.symbol ELSE research_news.symbol END,
            source = excluded.source,
            provider = CASE WHEN research_news.provider = '' THEN excluded.provider ELSE research_news.provider END,
            headline = excluded.headline,
            summary = excluded.summary,
            image_url = CASE WHEN research_news.image_url = '' THEN excluded.image_url ELSE research_news.image_url END,
            sentiment = CASE WHEN research_news.sentiment = '' THEN excluded.sentiment ELSE research_news.sentiment END,
            sentiment_score = CASE WHEN research_news.sentiment_score = 0.0 THEN excluded.sentiment_score ELSE research_news.sentiment_score END,
            tickers_json = excluded.tickers_json,
            categories_json = excluded.categories_json,
            body = CASE WHEN research_news.body = '' THEN excluded.body ELSE research_news.body END,
            updated_at = excluded.updated_at",
        params![
            a.url_hash, a.symbol.to_uppercase(), a.source, a.provider, a.headline, a.summary,
            a.url, a.published_at, a.image_url, a.sentiment, a.sentiment_score,
            tickers_json, categories_json, now_ts(), a.body,
        ],
    ).map_err(|e| format!("upsert news: {e}"))?;

    // FTS5 mirror — DELETE then INSERT for upsert semantics.
    let _ = conn.execute(
        "DELETE FROM research_news_fts WHERE url_hash = ?1",
        params![a.url_hash],
    );
    let _ = conn.execute(
        "INSERT INTO research_news_fts(url_hash, headline, summary, body) VALUES (?1,?2,?3,?4)",
        params![a.url_hash, a.headline, a.summary, a.body],
    );
    Ok(())
}

/// Permanently remove a news article and remember its hash so it is never
/// re-ingested. Backs the per-article "Remove / Ignore" action used to kill
/// GDELT false-positives (e.g. WOK cooking-recipe spam matched on the substring).
pub fn delete_news(conn: &Connection, url_hash: &str, symbol: &str) -> Result<(), String> {
    let _ = create_news_tables(conn);
    if url_hash.is_empty() {
        return Err("delete_news: url_hash empty".into());
    }
    conn.execute(
        "DELETE FROM research_news WHERE url_hash = ?1",
        params![url_hash],
    )
    .map_err(|e| format!("delete research_news: {e}"))?;
    let _ = conn.execute(
        "DELETE FROM research_news_fts WHERE url_hash = ?1",
        params![url_hash],
    );
    conn.execute(
        "INSERT OR REPLACE INTO research_news_ignored (url_hash, symbol, ignored_at)
         VALUES (?1, ?2, ?3)",
        params![url_hash, symbol.to_uppercase(), now_ts()],
    )
    .map_err(|e| format!("insert research_news_ignored: {e}"))?;
    Ok(())
}

/// Write the full article body for an existing row and refresh its FTS5
/// mirror so search hits the new content. Idempotent; safe to call from a
/// background hydration task.
pub fn upsert_news_body(conn: &Connection, url_hash: &str, body: &str) -> Result<(), String> {
    upsert_news_body_and_image(conn, url_hash, body, "")
}

/// Like `upsert_news_body` but also writes `image_url` when the caller
/// extracted a hero image (og:image / twitter:image) from the article
/// page. Only fills in the image when the row currently has none — we
/// don't want a body-fetch backfill to clobber an image URL the source
/// RSS feed already provided. Empty `image_url` is a no-op for that
/// column, so the existing `upsert_news_body` semantics are unchanged.
pub fn upsert_news_body_and_image(
    conn: &Connection,
    url_hash: &str,
    body: &str,
    image_url: &str,
) -> Result<(), String> {
    let _ = create_news_tables(conn);
    if url_hash.is_empty() || body.is_empty() {
        return Ok(());
    }
    conn.execute(
        "UPDATE research_news
            SET body = ?1,
                body_fetched_at = ?2,
                updated_at = ?2,
                image_url = CASE WHEN image_url = '' AND ?3 <> '' THEN ?3 ELSE image_url END
          WHERE url_hash = ?4",
        params![body, now_ts(), image_url, url_hash],
    )
    .map_err(|e| format!("update news body: {e}"))?;
    // FTS5: refresh headline+summary+body for this row. We need the headline
    // and summary because FTS rows are replaced, not patched in place.
    if let Ok((headline, summary)) = conn.query_row(
        "SELECT headline, summary FROM research_news WHERE url_hash = ?1",
        params![url_hash],
        |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
    ) {
        let _ = conn.execute(
            "DELETE FROM research_news_fts WHERE url_hash = ?1",
            params![url_hash],
        );
        let _ = conn.execute(
            "INSERT INTO research_news_fts(url_hash, headline, summary, body) VALUES (?1,?2,?3,?4)",
            params![url_hash, headline, summary, body],
        );
    }
    Ok(())
}

/// Return up to `limit` `(url_hash, url)` rows for articles whose body
/// hasn't been fetched yet AND whose `body_fetch_attempts` counter is below
/// [`MAX_BODY_FETCH_ATTEMPTS`]. Caller is the body hydrator — it walks the
/// list, fetches each URL, and writes the result via [`upsert_news_body`]
/// or [`bump_news_body_fetch_attempts`] for failures. Skipping URLs that
/// have already failed N times means a single publisher (e.g. Yahoo's JS-
/// only article pages) can't keep eating hydrator slots forever.
pub fn list_articles_missing_body(
    conn: &Connection,
    symbol: Option<&str>,
    limit: usize,
) -> Result<Vec<(String, String)>, String> {
    let _ = create_news_tables(conn);
    let limit_i = limit.min(10_000) as i64;
    let sym = symbol.map(|s| s.trim().to_uppercase()).unwrap_or_default();
    let rows: rusqlite::Result<Vec<(String, String)>> = if sym.is_empty() {
        let mut stmt = conn
            .prepare(
                "SELECT url_hash, url FROM research_news
                  WHERE body = '' AND url <> '' AND source <> 'SEC'
                    AND body_fetch_attempts < ?2
                  ORDER BY published_at DESC
                  LIMIT ?1",
            )
            .map_err(|e| format!("prepare missing body: {e}"))?;
        stmt.query_map(params![limit_i, MAX_BODY_FETCH_ATTEMPTS], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| format!("query missing body: {e}"))?
        .collect()
    } else {
        let like = format!("%\"{}\"%", sym);
        let mut stmt = conn
            .prepare(
                "SELECT url_hash, url FROM research_news
                  WHERE body = '' AND url <> '' AND source <> 'SEC'
                    AND body_fetch_attempts < ?4
                    AND (symbol = ?1 OR tickers_json LIKE ?2)
                  ORDER BY published_at DESC
                  LIMIT ?3",
            )
            .map_err(|e| format!("prepare missing body: {e}"))?;
        stmt.query_map(params![sym, like, limit_i, MAX_BODY_FETCH_ATTEMPTS], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| format!("query missing body: {e}"))?
        .collect()
    };
    rows.map_err(|e| format!("collect missing body: {e}"))
}

/// Increment the failure counter for a URL whose body fetch came back empty
/// or failed. Once it reaches [`MAX_BODY_FETCH_ATTEMPTS`] the row is filtered
/// out of [`list_articles_missing_body`] and the UI swaps "still hydrating"
/// for "body unavailable".
pub fn bump_news_body_fetch_attempts(conn: &Connection, url_hash: &str) -> Result<(), String> {
    if url_hash.is_empty() {
        return Ok(());
    }
    conn.execute(
        "UPDATE research_news
            SET body_fetch_attempts = body_fetch_attempts + 1,
                updated_at = ?1
          WHERE url_hash = ?2",
        params![now_ts(), url_hash],
    )
    .map(|_| ())
    .map_err(|e| format!("bump body attempts: {e}"))
}

/// HTTP-fetch `url` and return its plain-text article body. Caps the
/// fetched HTML at `MAX_FETCH_BYTES` so a hostile or runaway page can't
/// Normalize a headline for content-based dedup across sources. Strips the
/// publisher suffix patterns that local-news sites tack on (`" | <Publisher>"`,
/// `" - <Publisher>"`) and lowercases / collapses whitespace. Two articles
/// that produce the same normalized headline are considered the same story
/// surfaced by different outlets, so the UI can show one row with a
/// "N sources" badge instead of N near-identical rows.
///
/// Examples (all collapse to "dads club colchester says it is the antidote
/// to manosphere"):
///   - "Dads club Colchester says it is the antidote to manosphere"
///   - "Dads club Colchester says it is the antidote to manosphere | Clacton and Frinton Gazette"
///   - "Dads club Colchester says it is the antidote to manosphere - Halstead Gazette"
pub fn normalize_headline_for_dedup(headline: &str) -> String {
    let lower = headline.to_lowercase();
    // Strip trailing " | <publisher>" — the most common pattern across
    // Yahoo-syndicated UK regional news. `rsplit_once` so we don't break
    // headlines that legitimately contain "|" earlier in the text.
    let trimmed: &str = match lower.rsplit_once(" | ") {
        // Sanity check: the prefix must be substantial (> 12 chars) so we
        // don't decapitate a headline like "Apple | Q3 earnings".
        Some((before, _)) if before.len() > 12 => before,
        _ => &lower,
    };
    // Also try " - <publisher>" (Yahoo-syndicated US local news pattern).
    // Same length guard so common em-dash titles aren't truncated.
    let trimmed: &str = match trimmed.rsplit_once(" - ") {
        Some((before, _)) if before.len() > 12 => before,
        _ => trimmed,
    };
    trimmed.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Group articles whose normalized headlines match into a single
/// "story" with one primary (the most recent) and the rest as
/// alternates. Preserves the input ordering of groups (the order their
/// primary article first appears in the input). Within a group the
/// primary is the article with the latest `published_at`; alternates
/// are sorted newest → oldest after the primary. Returns
/// `Vec<(primary_index, alternate_indices)>` so the caller can index
/// back into the original slice without cloning the articles.
pub fn group_articles_by_headline(articles: &[NewsArticle]) -> Vec<(usize, Vec<usize>)> {
    use std::collections::HashMap;
    // Map normalized headline → indices in input order.
    let mut buckets: HashMap<String, Vec<usize>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for (i, a) in articles.iter().enumerate() {
        let key = normalize_headline_for_dedup(&a.headline);
        if !buckets.contains_key(&key) {
            order.push(key.clone());
        }
        buckets.entry(key).or_default().push(i);
    }
    let mut out: Vec<(usize, Vec<usize>)> = Vec::with_capacity(order.len());
    for key in order {
        let mut indices = buckets.remove(&key).unwrap_or_default();
        // Sort by published_at descending so the freshest source wins
        // the primary slot.
        indices.sort_by(|&a, &b| articles[b].published_at.cmp(&articles[a].published_at));
        let primary = indices.remove(0);
        out.push((primary, indices));
    }
    out
}

/// Bulk upsert a batch in a single transaction.
pub fn upsert_news_batch(conn: &Connection, articles: &[NewsArticle]) -> Result<usize, String> {
    let _ = create_news_tables(conn);
    let mut count = 0;
    let _ = conn.execute_batch("BEGIN IMMEDIATE");
    for a in articles {
        match upsert_news(conn, a) {
            Ok(()) => count += 1,
            Err(e) => tracing::warn!("news upsert skip: {e}"),
        }
    }
    let _ = conn.execute_batch("COMMIT");
    Ok(count)
}

pub fn news_cache_is_fresh(
    conn: &Connection,
    symbol: &str,
    max_age_secs: i64,
    min_articles: usize,
) -> Result<bool, String> {
    let _ = create_news_tables(conn);
    let sym = symbol.trim().to_uppercase();
    if sym.is_empty() {
        return Ok(false);
    }
    let now = now_ts();
    let row: Option<(i64, i64)> = conn
        .query_row(
            "SELECT last_scrape_at, article_count FROM research_news_scrape_index WHERE symbol = ?1",
            params![sym],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()
        .map_err(|e| format!("query news scrape index: {e}"))?;
    let Some((last_scrape_at, article_count)) = row else {
        return Ok(false);
    };
    Ok(last_scrape_at > 0
        && now.saturating_sub(last_scrape_at) < max_age_secs
        && article_count as usize >= min_articles)
}

pub fn mark_news_scraped(conn: &Connection, symbol: &str) -> Result<usize, String> {
    let _ = create_news_tables(conn);
    let sym = symbol.trim().to_uppercase();
    if sym.is_empty() {
        return Ok(0);
    }
    let count: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM research_news WHERE source <> 'SEC' AND symbol = ?1",
            params![sym],
            |r| r.get(0),
        )
        .unwrap_or(0);
    conn.execute(
        "INSERT INTO research_news_scrape_index (symbol, last_scrape_at, article_count)
         VALUES (?1, ?2, ?3)
         ON CONFLICT(symbol) DO UPDATE SET
            last_scrape_at = excluded.last_scrape_at,
            article_count = excluded.article_count",
        params![sym, now_ts(), count],
    )
    .map_err(|e| format!("mark news scraped: {e}"))?;
    Ok(count as usize)
}

pub fn fresh_news_symbols(
    conn: &Connection,
    symbols: &[String],
    max_age_secs: i64,
    min_articles: usize,
) -> Result<std::collections::HashSet<String>, String> {
    let _ = create_news_tables(conn);
    let mut unique: Vec<String> = symbols
        .iter()
        .map(|s| s.trim().to_uppercase())
        .filter(|s| !s.is_empty())
        .collect();
    unique.sort_unstable();
    unique.dedup();
    if unique.is_empty() {
        return Ok(std::collections::HashSet::new());
    }

    let cutoff = now_ts().saturating_sub(max_age_secs);
    let placeholders = std::iter::repeat("?")
        .take(unique.len())
        .collect::<Vec<_>>()
        .join(",");
    let sql = format!(
        "SELECT symbol FROM research_news_scrape_index
         WHERE last_scrape_at >= ?1 AND article_count >= ?2 AND symbol IN ({placeholders})"
    );
    let mut values: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::with_capacity(unique.len() + 2);
    values.push(Box::new(cutoff));
    values.push(Box::new(min_articles as i64));
    for sym in unique {
        values.push(Box::new(sym));
    }
    let params_refs: Vec<&dyn rusqlite::types::ToSql> = values.iter().map(|v| v.as_ref()).collect();
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("prepare fresh news symbols: {e}"))?;
    let rows = stmt
        .query_map(params_refs.as_slice(), |row| row.get::<_, String>(0))
        .map_err(|e| format!("query fresh news symbols: {e}"))?;
    let mut out = std::collections::HashSet::new();
    for row in rows {
        if let Ok(sym) = row {
            out.insert(sym);
        }
    }
    Ok(out)
}

/// Fetch the most recent N cached articles for a symbol (empty symbol matches anything).
pub fn get_news_by_symbol(
    conn: &Connection,
    symbol: &str,
    limit: usize,
) -> Result<Vec<NewsArticle>, String> {
    let _ = create_news_tables(conn);
    let sym = symbol.to_uppercase();
    let requested_symbols: Vec<String> = sym
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
        .collect();
    if requested_symbols.len() > 1 {
        let mut by_hash = std::collections::BTreeMap::new();
        for requested in &requested_symbols {
            for article in get_news_by_symbol(conn, requested, limit)? {
                by_hash.entry(article.url_hash.clone()).or_insert(article);
            }
        }
        let mut out: Vec<NewsArticle> = by_hash.into_values().collect();
        out.sort_by(|a, b| b.published_at.cmp(&a.published_at));
        out.truncate(limit);
        return Ok(out);
    }
    let sql = if sym.is_empty() {
        "SELECT url_hash, symbol, source, provider, headline, summary, url, published_at,
                image_url, sentiment, sentiment_score, tickers_json, categories_json, body,
                body_fetch_attempts
         FROM research_news
         WHERE source <> 'SEC'
         ORDER BY published_at DESC, updated_at DESC
         LIMIT ?1"
    } else {
        "SELECT url_hash, symbol, source, provider, headline, summary, url, published_at,
                image_url, sentiment, sentiment_score, tickers_json, categories_json, body,
                body_fetch_attempts
         FROM research_news
         WHERE source <> 'SEC' AND (symbol = ?2 OR tickers_json LIKE ?3)
         ORDER BY published_at DESC, updated_at DESC
         LIMIT ?1"
    };
    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| format!("prepare get_news: {e}"))?;
    let lim = limit as i64;
    let mut rows = if sym.is_empty() {
        stmt.query(params![lim])
            .map_err(|e| format!("query get_news: {e}"))?
    } else {
        let like = format!("%\"{}\"%", sym);
        stmt.query(params![lim, sym, like])
            .map_err(|e| format!("query get_news: {e}"))?
    };
    let mut out = Vec::new();
    while let Some(r) = rows.next().map_err(|e| format!("row get_news: {e}"))? {
        let tickers_s: String = r.get(11).unwrap_or_default();
        let cats_s: String = r.get(12).unwrap_or_default();
        out.push(NewsArticle {
            url_hash: r.get(0).unwrap_or_default(),
            symbol: r.get(1).unwrap_or_default(),
            source: r.get(2).unwrap_or_default(),
            provider: r.get(3).unwrap_or_default(),
            headline: r.get(4).unwrap_or_default(),
            summary: r.get(5).unwrap_or_default(),
            url: r.get(6).unwrap_or_default(),
            published_at: r.get(7).unwrap_or(0),
            image_url: r.get(8).unwrap_or_default(),
            sentiment: r.get(9).unwrap_or_default(),
            sentiment_score: r.get(10).unwrap_or(0.0),
            tickers: serde_json::from_str(&tickers_s).unwrap_or_default(),
            categories: serde_json::from_str(&cats_s).unwrap_or_default(),
            body: r.get(13).unwrap_or_default(),
            body_fetch_attempts: r.get(14).unwrap_or(0),
        });
    }
    Ok(out)
}

/// Full-text search over cached news via FTS5.
pub fn search_news(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<NewsArticle>, String> {
    let _ = create_news_tables(conn);
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let safe_query = fts5_safe_query(query);
    let primary = search_news_fts(conn, query, limit);
    if primary.is_ok() || safe_query == query.trim() {
        return primary;
    }
    search_news_fts(conn, &safe_query, limit)
}

fn fts5_safe_query(query: &str) -> String {
    let terms: Vec<String> = query
        .split(|ch: char| ch == ',' || ch.is_ascii_whitespace())
        .map(str::trim)
        .filter(|term| !term.is_empty())
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect();
    if terms.is_empty() {
        query.trim().to_string()
    } else {
        terms.join(" OR ")
    }
}

fn search_news_fts(
    conn: &Connection,
    query: &str,
    limit: usize,
) -> Result<Vec<NewsArticle>, String> {
    let mut stmt = conn.prepare(
        "SELECT n.url_hash, n.symbol, n.source, n.provider, n.headline, n.summary, n.url, n.published_at,
                n.image_url, n.sentiment, n.sentiment_score, n.tickers_json, n.categories_json, n.body,
                n.body_fetch_attempts
         FROM research_news n
         JOIN research_news_fts f ON f.url_hash = n.url_hash
         WHERE n.source <> 'SEC' AND research_news_fts MATCH ?1
         ORDER BY n.published_at DESC
         LIMIT ?2"
    ).map_err(|e| format!("prepare search_news: {e}"))?;
    let mut rows = stmt
        .query(params![query, limit as i64])
        .map_err(|e| format!("query search_news: {e}"))?;
    let mut out = Vec::new();
    while let Some(r) = rows.next().map_err(|e| format!("row search_news: {e}"))? {
        let tickers_s: String = r.get(11).unwrap_or_default();
        let cats_s: String = r.get(12).unwrap_or_default();
        out.push(NewsArticle {
            url_hash: r.get(0).unwrap_or_default(),
            symbol: r.get(1).unwrap_or_default(),
            source: r.get(2).unwrap_or_default(),
            provider: r.get(3).unwrap_or_default(),
            headline: r.get(4).unwrap_or_default(),
            summary: r.get(5).unwrap_or_default(),
            url: r.get(6).unwrap_or_default(),
            published_at: r.get(7).unwrap_or(0),
            image_url: r.get(8).unwrap_or_default(),
            sentiment: r.get(9).unwrap_or_default(),
            sentiment_score: r.get(10).unwrap_or(0.0),
            tickers: serde_json::from_str(&tickers_s).unwrap_or_default(),
            categories: serde_json::from_str(&cats_s).unwrap_or_default(),
            body: r.get(13).unwrap_or_default(),
            body_fetch_attempts: r.get(14).unwrap_or(0),
        });
    }
    Ok(out)
}

/// Cheap total-row count for the `research_news` table. Used by the
/// News window header to display the on-disk article count even when
/// the in-memory list is empty (e.g. fresh launch, before the user
/// clicks Load Cached). Indexed PK scan — sub-millisecond on the
/// production cache.
pub fn count_all_articles(conn: &Connection) -> Result<i64, String> {
    let _ = create_news_tables(conn);
    conn.query_row("SELECT COUNT(*) FROM research_news", [], |r| {
        r.get::<_, i64>(0)
    })
    .map_err(|e| format!("count all articles: {e}"))
}

/// DDL-free article count for UI hot paths.
///
/// Unlike [`count_all_articles`], this never issues `CREATE TABLE`, so it is
/// safe to run on the dedicated read-only connection (`Cache::try_connection`).
/// The News header's "N in DB" refresh previously called `count_all_articles`
/// through the *write* connection on the render thread; that grabbed the same
/// mutex the bulk bar-sync writers hold, so it blocked for the whole in-flight
/// OHLC-sweep transaction — the 10–17s News-window frame stalls that recurred
/// once per sweep cycle. Counting on the read connection reads the committed
/// WAL snapshot without ever waiting on a writer. A not-yet-created table
/// (fresh cache) returns 0 rather than erroring, since this is a best-effort
/// header count.
pub fn count_all_articles_readonly(conn: &Connection) -> Result<i64, String> {
    let table_exists = conn
        .query_row(
            "SELECT 1 FROM sqlite_master WHERE type='table' AND name='research_news'",
            [],
            |_| Ok(()),
        )
        .is_ok();
    if !table_exists {
        return Ok(0);
    }
    conn.query_row("SELECT COUNT(*) FROM research_news", [], |r| {
        r.get::<_, i64>(0)
    })
    .map_err(|e| format!("count all articles (ro): {e}"))
}

/// Count articles whose `published_at` is older than `cutoff_ts`. Paired
/// with `purge_older_than` for the Storage Manager UI: the count gives
/// the user a preview ("N articles would be deleted") before the
/// destructive button.
pub fn count_articles_older_than(conn: &Connection, cutoff_ts: i64) -> Result<i64, String> {
    let _ = create_news_tables(conn);
    conn.query_row(
        "SELECT COUNT(*) FROM research_news WHERE published_at < ?1",
        params![cutoff_ts],
        |r| r.get::<_, i64>(0),
    )
    .map_err(|e| format!("count older than: {e}"))
}

/// Delete articles older than `cutoff_ts`. Keeps the FTS5 table in sync.
pub fn purge_older_than(conn: &Connection, cutoff_ts: i64) -> Result<usize, String> {
    let _ = create_news_tables(conn);
    let hashes: Vec<String> = {
        let mut stmt = conn
            .prepare("SELECT url_hash FROM research_news WHERE published_at < ?1")
            .map_err(|e| format!("prepare purge select: {e}"))?;
        let mut rows = stmt
            .query(params![cutoff_ts])
            .map_err(|e| format!("query purge: {e}"))?;
        let mut v = Vec::new();
        while let Some(r) = rows.next().map_err(|e| format!("row purge: {e}"))? {
            v.push(r.get::<_, String>(0).unwrap_or_default());
        }
        v
    };
    for h in &hashes {
        let _ = conn.execute("DELETE FROM research_news WHERE url_hash = ?1", params![h]);
        let _ = conn.execute(
            "DELETE FROM research_news_fts WHERE url_hash = ?1",
            params![h],
        );
    }
    Ok(hashes.len())
}

/// Cap `research_news` at `max_rows` by deleting the oldest articles beyond the
/// newest `max_rows`. Keeps the FTS5 mirror in sync. Paired with
/// [`purge_older_than`]: age-based retention can't bound a burst of
/// full-universe scraping that lands inside the retention window, so this row
/// cap is the hard ceiling that keeps COUNT(*)/FTS search and the on-disk
/// footprint bounded. No-op when the table already holds `max_rows` or fewer
/// (or when `max_rows` is negative). Ordering matches the UI's newest-first
/// view: `published_at DESC, rowid DESC`, so a tie on timestamp drops the
/// earlier-inserted row first.
pub fn enforce_max_rows(conn: &Connection, max_rows: i64) -> Result<usize, String> {
    if max_rows < 0 {
        return Ok(0);
    }
    let _ = create_news_tables(conn);
    let hashes: Vec<String> = {
        let mut stmt = conn
            .prepare(
                "SELECT url_hash FROM research_news \
                 ORDER BY published_at DESC, rowid DESC \
                 LIMIT -1 OFFSET ?1",
            )
            .map_err(|e| format!("prepare cap select: {e}"))?;
        let mut rows = stmt
            .query(params![max_rows])
            .map_err(|e| format!("query cap: {e}"))?;
        let mut v = Vec::new();
        while let Some(r) = rows.next().map_err(|e| format!("row cap: {e}"))? {
            v.push(r.get::<_, String>(0).unwrap_or_default());
        }
        v
    };
    for h in &hashes {
        let _ = conn.execute("DELETE FROM research_news WHERE url_hash = ?1", params![h]);
        let _ = conn.execute(
            "DELETE FROM research_news_fts WHERE url_hash = ?1",
            params![h],
        );
    }
    Ok(hashes.len())
}

// ── Bulk scrape helper ────────────────────────────────────────────────────

/// Optional API keys for the news aggregation pipeline. All fields default to
/// the empty string; sources whose keys are blank are skipped silently.
#[derive(Clone, Default, Debug)]
pub struct NewsApiKeys {
    pub marketaux: String,
    pub alpha_vantage: String,
    pub fmp: String,
    pub finnhub: String,
    pub cryptopanic: String,
}

/// Fetch news for a symbol from all configured sources. Routes between equity
/// and crypto-native source sets via [`is_crypto_symbol`]; equity sources are
/// not called for crypto pairs because they return nothing and waste rate-limit
/// budget.
///
/// Sources without keys are skipped silently. Rate-limits with sleeps between calls.
///
/// This function is pure-async with no SQLite dependency — callers must upsert
/// the returned vector themselves via `upsert_news_batch`. Splitting fetch from
/// persistence lets us call this safely from a tokio task without dragging a
/// non-Send `&rusqlite::Connection` across await boundaries.
pub async fn fetch_all_sources_for_symbol(
    client: &reqwest::Client,
    symbol: &str,
    keys: &NewsApiKeys,
    mut cb: impl FnMut(&str),
) -> Result<Vec<NewsArticle>, String> {
    let sym = symbol.to_uppercase();
    if sym.is_empty() {
        return Err("empty symbol".into());
    }

    let mut all_articles: Vec<NewsArticle> = Vec::new();

    // GDELT (no key) — common to both flows. Skip silently while a prior 429
    // cooldown is active so bulk per-symbol loops don't generate one failure
    // log per ticker. For crypto, query by the asset's full name when known
    // so headlines about "Bitcoin" surface even when the ticker isn't quoted.
    if !gdelt_in_cooldown() {
        let gdelt_query = crypto_base_for_symbol(&sym)
            .and_then(|b| crypto_full_name(&b).map(|s| s.to_string()))
            .unwrap_or_else(|| sym.clone());
        match fetch_gdelt_news(client, &gdelt_query, 30).await {
            Ok(v) => {
                cb(&format!("news/gdelt {}: {} articles", gdelt_query, v.len()));
                all_articles.extend(v);
            }
            Err(e) => {
                cb(&format!("news/gdelt {} failed: {e}", gdelt_query));
                if gdelt_in_cooldown() {
                    cb(&format!(
                        "news/gdelt: rate-limited, skipping for {}s",
                        gdelt_cooldown_remaining_secs()
                    ));
                }
            }
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    }

    if is_crypto_symbol(&sym) {
        // ── Crypto-native sources ─────────────────────────────────────────
        // Yahoo also serves crypto via `BTC-USD`-style symbols. Re-format the
        // base so it actually returns results instead of the dead `BTCUSD`.
        if let Some(base) = crypto_base_for_symbol(&sym) {
            let yahoo_sym = format!("{}-USD", base);
            match fetch_yahoo_rss(client, &yahoo_sym).await {
                Ok(v) => {
                    cb(&format!(
                        "news/yahoo_rss {}: {} articles",
                        yahoo_sym,
                        v.len()
                    ));
                    all_articles.extend(v);
                }
                Err(e) => cb(&format!("news/yahoo_rss {} failed: {e}", yahoo_sym)),
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }

        if !keys.cryptopanic.is_empty() {
            match fetch_cryptopanic_news(client, &sym, &keys.cryptopanic).await {
                Ok(v) => {
                    cb(&format!("news/cryptopanic {}: {} articles", sym, v.len()));
                    all_articles.extend(v);
                }
                Err(e) => cb(&format!("news/cryptopanic {} failed: {e}", sym)),
            }
            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
        }

        match fetch_coindesk_rss(client, &sym).await {
            Ok(v) => {
                cb(&format!("news/coindesk {}: {} articles", sym, v.len()));
                all_articles.extend(v);
            }
            Err(e) => cb(&format!("news/coindesk {} failed: {e}", sym)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        if !keys.finnhub.is_empty() {
            match fetch_finnhub_crypto_news(client, &sym, &keys.finnhub).await {
                Ok(v) => {
                    cb(&format!(
                        "news/finnhub_crypto {}: {} articles",
                        sym,
                        v.len()
                    ));
                    all_articles.extend(v);
                }
                Err(e) => cb(&format!("news/finnhub_crypto {} failed: {e}", sym)),
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    } else {
        // ── Equity / general sources ──────────────────────────────────────
        match fetch_yahoo_rss(client, &sym).await {
            Ok(v) => {
                cb(&format!("news/yahoo_rss {}: {} articles", sym, v.len()));
                all_articles.extend(v);
            }
            Err(e) => cb(&format!("news/yahoo_rss {} failed: {e}", sym)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;

        // SEC EDGAR filings stay in the dedicated SEC/insider/research tools, not
        // the general News window. Keep this feed out of multi-source news so
        // routine filings do not drown out actual market news.

        if !keys.marketaux.is_empty() {
            match fetch_marketaux_news(client, &sym, &keys.marketaux).await {
                Ok(v) => {
                    cb(&format!("news/marketaux {}: {} articles", sym, v.len()));
                    all_articles.extend(v);
                }
                Err(e) => cb(&format!("news/marketaux {} failed: {e}", sym)),
            }
            tokio::time::sleep(std::time::Duration::from_millis(900)).await;
        }

        if !keys.alpha_vantage.is_empty() {
            match fetch_alpha_vantage_news(client, &sym, &keys.alpha_vantage).await {
                Ok(v) => {
                    cb(&format!("news/alpha_vantage {}: {} articles", sym, v.len()));
                    all_articles.extend(v);
                }
                Err(e) => cb(&format!("news/alpha_vantage {} failed: {e}", sym)),
            }
            tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
        }

        if !keys.fmp.is_empty() {
            match fetch_fmp_news(client, &sym, &keys.fmp).await {
                Ok(v) => {
                    cb(&format!("news/fmp {}: {} articles", sym, v.len()));
                    all_articles.extend(v);
                }
                Err(e) => cb(&format!("news/fmp {} failed: {e}", sym)),
            }
            tokio::time::sleep(std::time::Duration::from_millis(300)).await;
        }
    }

    cb(&format!(
        "news/{}: {} articles fetched",
        sym,
        all_articles.len()
    ));
    // Post-ingest relevance gate for short tickers (prevents WOK cooking spam etc.)
    let filtered: Vec<NewsArticle> = all_articles
        .into_iter()
        .filter(|a| article_is_relevant_for_ticker(a, &sym))
        .collect();

    Ok(filtered)
}

/// Heuristic — treat alphabetic ticker ≤5 chars as US-listed (EDGAR eligible).
#[cfg(test)]
fn is_us_symbol(s: &str) -> bool {
    !s.is_empty() && s.len() <= 5 && s.chars().all(|c| c.is_ascii_alphabetic() || c == '.')
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests;
