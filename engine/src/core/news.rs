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
// we see a 429 we park GDELT for `RATE_LIMIT_COOLDOWN_SECS` and callers
// short-circuit via `gdelt_in_cooldown()` instead of issuing the request.

const RATE_LIMIT_COOLDOWN_SECS: i64 = 600;
static GDELT_COOLDOWN_UNTIL: AtomicI64 = AtomicI64::new(0);

fn now_secs() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Seconds remaining on the GDELT 429 cooldown, or 0 if not throttled.
pub fn gdelt_cooldown_remaining_secs() -> i64 {
    (GDELT_COOLDOWN_UNTIL.load(Ordering::Relaxed) - now_secs()).max(0)
}

/// True while GDELT requests should be skipped after a 429.
pub fn gdelt_in_cooldown() -> bool {
    gdelt_cooldown_remaining_secs() > 0
}

fn trip_gdelt_cooldown() {
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

/// Write the full article body for an existing row and refresh its FTS5
/// mirror so search hits the new content. Idempotent; safe to call from a
/// background hydration task.
pub fn upsert_news_body(conn: &Connection, url_hash: &str, body: &str) -> Result<(), String> {
    let _ = create_news_tables(conn);
    if url_hash.is_empty() || body.is_empty() {
        return Ok(());
    }
    conn.execute(
        "UPDATE research_news
            SET body = ?1, body_fetched_at = ?2, updated_at = ?2
          WHERE url_hash = ?3",
        params![body, now_ts(), url_hash],
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
        stmt.query_map(
            params![sym, like, limit_i, MAX_BODY_FETCH_ATTEMPTS],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
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
/// blow up the cache. Returns `None` for non-2xx responses, empty bodies,
/// or extracted text under 200 chars (likely a paywall splash, not the
/// article). The output is suitable for direct storage via
/// [`upsert_news_body`] and for indexing in `research_news_fts`.
pub async fn fetch_article_body(url: &str) -> Option<String> {
    const MAX_FETCH_BYTES: usize = 2 * 1024 * 1024; // 2 MiB cap on raw HTML
    const MIN_BODY_CHARS: usize = 200; // anything shorter is probably a redirect/paywall splash
    const FETCH_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(20);
    let url = url.trim();
    if url.is_empty() || !(url.starts_with("http://") || url.starts_with("https://")) {
        return None;
    }
    let client = reqwest::Client::builder()
        .user_agent(
            "Mozilla/5.0 (compatible; TyphooN-Terminal/0.1; +https://riskprivacy.com/typhoon)",
        )
        .timeout(FETCH_TIMEOUT)
        .build()
        .ok()?;
    let resp = client.get(url).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    // Soft size cap: stream-read until we hit the limit, then bail.
    let bytes = resp.bytes().await.ok()?;
    let html = if bytes.len() > MAX_FETCH_BYTES {
        String::from_utf8_lossy(&bytes[..MAX_FETCH_BYTES]).into_owned()
    } else {
        String::from_utf8_lossy(&bytes).into_owned()
    };
    let text = extract_article_text(&html);
    if text.chars().count() < MIN_BODY_CHARS {
        return None;
    }
    Some(text)
}

/// Light-weight HTML → text extractor. Drops `<script>`/`<style>`/`<noscript>`
/// blocks wholesale, strips remaining tags, decodes the common entities, and
/// collapses whitespace. Not a real parser — designed for the constrained
/// case of news article body extraction where the publisher's main content
/// is broadly readable as plain text once tags are removed.
///
/// Operates on bytes (so byte indices for the drop-tag lookup stay sound)
/// and writes raw bytes into a `Vec<u8>` so UTF-8 sequences for non-ASCII
/// characters (smart quotes, em dashes, accented letters) survive intact.
/// The final `from_utf8_lossy` only replaces genuinely invalid bytes.
pub fn extract_article_text(html: &str) -> String {
    let bytes = html.as_bytes();
    let lower: Vec<u8> = bytes.iter().map(|b| b.to_ascii_lowercase()).collect();
    let mut out: Vec<u8> = Vec::with_capacity(html.len() / 4);

    fn find_close(lower: &[u8], from: usize, tag: &[u8]) -> Option<usize> {
        let mut i = from;
        while i + tag.len() <= lower.len() {
            if lower[i..i + tag.len()] == *tag {
                return Some(i);
            }
            i += 1;
        }
        None
    }

    let drop_tags: [(&[u8], &[u8]); 3] = [
        (b"<script", b"</script>"),
        (b"<style", b"</style>"),
        (b"<noscript", b"</noscript>"),
    ];

    let mut i = 0;
    let mut inside_tag = false;
    while i < bytes.len() {
        // Skip whole drop-block (script/style/noscript) if we're at one.
        if !inside_tag {
            let mut skipped = false;
            for (open, close) in &drop_tags {
                if i + open.len() <= lower.len() && &lower[i..i + open.len()] == *open {
                    if let Some(end) = find_close(&lower, i + open.len(), close) {
                        i = end + close.len();
                        skipped = true;
                        break;
                    } else {
                        // Unterminated; bail out of the rest of the doc.
                        return finalize_extracted_text(out);
                    }
                }
            }
            if skipped {
                continue;
            }
        }

        let b = bytes[i];
        if b == b'<' {
            inside_tag = true;
        } else if b == b'>' {
            inside_tag = false;
            // Treat block-level closers as paragraph breaks so we don't
            // glue adjacent paragraphs into one blob.
            out.push(b' ');
        } else if !inside_tag {
            out.push(b);
        }
        i += 1;
    }

    finalize_extracted_text(out)
}

fn finalize_extracted_text(raw_bytes: Vec<u8>) -> String {
    // Lossy decode here is final — anything that wasn't valid UTF-8 in the
    // source HTML gets a replacement char, which is fine for an indexable
    // article body.
    let raw = String::from_utf8_lossy(&raw_bytes).into_owned();
    let decoded = decode_html_entities(&raw);
    // Collapse whitespace runs to single spaces; convert paragraph breaks
    // (multiple spaces) into newlines so the stored text is readable.
    let mut out = String::with_capacity(decoded.len());
    let mut last_was_space = true;
    let mut consecutive_spaces = 0u32;
    for ch in decoded.chars() {
        if ch.is_whitespace() {
            consecutive_spaces += 1;
            if !last_was_space {
                out.push(if consecutive_spaces > 4 { '\n' } else { ' ' });
                last_was_space = true;
            } else if consecutive_spaces == 5 {
                // first promotion of a long run → make it a paragraph break
                if out.ends_with(' ') {
                    out.pop();
                }
                out.push('\n');
            }
        } else {
            consecutive_spaces = 0;
            out.push(ch);
            last_was_space = false;
        }
    }
    out.trim().to_string()
}

fn decode_html_entities(s: &str) -> String {
    // Walk by chars but track byte position so we can spot `&...;` entities
    // (which are pure ASCII) without breaking up multi-byte UTF-8 sequences
    // in the surrounding text.
    let mut out = String::with_capacity(s.len());
    let bytes = s.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'&' {
            if let Some(semi) = (i + 1..(i + 12).min(bytes.len())).find(|&j| bytes[j] == b';') {
                let entity = &s[i + 1..semi];
                let mapped = match entity {
                    "amp" => Some("&"),
                    "lt" => Some("<"),
                    "gt" => Some(">"),
                    "quot" => Some("\""),
                    "apos" => Some("'"),
                    "nbsp" => Some(" "),
                    "mdash" => Some("—"),
                    "ndash" => Some("–"),
                    "hellip" => Some("…"),
                    "lsquo" | "rsquo" => Some("'"),
                    "ldquo" | "rdquo" => Some("\""),
                    "copy" => Some("©"),
                    "reg" => Some("®"),
                    _ => None,
                };
                if let Some(m) = mapped {
                    out.push_str(m);
                    i = semi + 1;
                    continue;
                }
                // Numeric: &#NNN; or &#xHH;
                if let Some(num) = entity.strip_prefix('#') {
                    let parsed = if let Some(hex) =
                        num.strip_prefix('x').or_else(|| num.strip_prefix('X'))
                    {
                        u32::from_str_radix(hex, 16).ok()
                    } else {
                        num.parse::<u32>().ok()
                    };
                    if let Some(code) = parsed.and_then(char::from_u32) {
                        out.push(code);
                        i = semi + 1;
                        continue;
                    }
                }
            }
        }
        // Determine the byte length of the char starting at `i` so we copy
        // the whole UTF-8 sequence in one shot.
        let len = utf8_char_len(bytes[i]);
        if i + len <= bytes.len() {
            if let Ok(seg) = std::str::from_utf8(&bytes[i..i + len]) {
                out.push_str(seg);
                i += len;
                continue;
            }
        }
        // Fallback for an invalid sequence: skip one byte and continue.
        i += 1;
    }
    out
}

fn utf8_char_len(b: u8) -> usize {
    if b < 0x80 {
        1
    } else if b < 0xC0 {
        1
    }
    // continuation; treat as 1 to make progress on malformed input
    else if b < 0xE0 {
        2
    } else if b < 0xF0 {
        3
    } else {
        4
    }
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

// ── Fetchers ──────────────────────────────────────────────────────────────

/// GDELT 2.0 Doc API — global news, no key required.
/// Returns articles mentioning the symbol or company name from the last 24h.
pub async fn fetch_gdelt_news(
    client: &reqwest::Client,
    query: &str,
    max_records: u32,
) -> Result<Vec<NewsArticle>, String> {
    if query.trim().is_empty() {
        return Ok(vec![]);
    }
    let remaining = gdelt_cooldown_remaining_secs();
    if remaining > 0 {
        return Err(format!("GDELT cooldown: {}s remaining", remaining));
    }
    let url = "https://api.gdeltproject.org/api/v2/doc/doc";
    let resp = client
        .get(url)
        .query(&[
            ("query", query),
            ("mode", "ArtList"),
            ("format", "json"),
            ("maxrecords", &max_records.to_string()),
            ("sort", "DateDesc"),
            ("timespan", "24h"),
        ])
        .header("User-Agent", "Mozilla/5.0 TyphooN-Terminal/0.1")
        .send()
        .await
        .map_err(|e| format!("GDELT request failed: {e}"))?;
    if !resp.status().is_success() {
        let code = resp.status().as_u16();
        if code == 429 {
            trip_gdelt_cooldown();
        }
        return Err(format!("GDELT: HTTP {}", resp.status()));
    }
    let text = resp.text().await.map_err(|e| format!("GDELT read: {e}"))?;
    // GDELT sometimes returns empty body on zero matches — treat as empty list.
    if text.trim().is_empty() {
        return Ok(vec![]);
    }
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        format!(
            "GDELT parse: {e}; body: {}",
            &text.chars().take(120).collect::<String>()
        )
    })?;
    let mut out = Vec::new();
    if let Some(arr) = v["articles"].as_array() {
        for e in arr {
            let url = e["url"].as_str().unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let title = e["title"].as_str().unwrap_or("").to_string();
            // GDELT timestamps are yyyymmddTHHMMSSZ
            let ts_raw = e["seendate"].as_str().unwrap_or("");
            let published_at = parse_gdelt_ts(ts_raw);
            let art = NewsArticle {
                symbol: query.to_uppercase(),
                source: "GDELT".into(),
                provider: e["domain"].as_str().unwrap_or("").to_string(),
                headline: title,
                summary: String::new(),
                url: url.clone(),
                published_at,
                image_url: e["socialimage"].as_str().unwrap_or("").to_string(),
                ..Default::default()
            }
            .with_hash();
            out.push(art);
        }
    }
    Ok(out)
}

fn parse_gdelt_ts(s: &str) -> i64 {
    // Format: "20260413T142030Z"
    if s.len() < 15 {
        return 0;
    }
    let y: i32 = s[0..4].parse().unwrap_or(0);
    let mo: u32 = s[4..6].parse().unwrap_or(0);
    let d: u32 = s[6..8].parse().unwrap_or(0);
    let h: u32 = s[9..11].parse().unwrap_or(0);
    let mi: u32 = s[11..13].parse().unwrap_or(0);
    let sc: u32 = s[13..15].parse().unwrap_or(0);
    chrono::NaiveDate::from_ymd_opt(y, mo, d)
        .and_then(|dt| dt.and_hms_opt(h, mi, sc))
        .map(|ndt| ndt.and_utc().timestamp())
        .unwrap_or(0)
}

/// Yahoo Finance RSS per-symbol feed — no key.
pub async fn fetch_yahoo_rss(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<Vec<NewsArticle>, String> {
    let clean = symbol.replace("/USD", "").replace("/", "").to_uppercase();
    if clean.is_empty() {
        return Ok(vec![]);
    }
    let url = format!(
        "https://feeds.finance.yahoo.com/rss/2.0/headline?s={}&region=US&lang=en-US",
        clean
    );
    let resp = client
        .get(&url)
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1",
        )
        .send()
        .await
        .map_err(|e| format!("Yahoo RSS request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Yahoo RSS: HTTP {}", resp.status()));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| format!("Yahoo RSS read: {e}"))?;
    Ok(parse_rss_items(&body, &clean, "YahooRSS"))
}

/// SEC EDGAR Atom feed for a single ticker's recent filings — used as a news proxy.
pub async fn fetch_sec_edgar_rss(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<Vec<NewsArticle>, String> {
    let sym = symbol.to_uppercase();
    if sym.is_empty() {
        return Ok(vec![]);
    }
    let url = format!(
        "https://www.sec.gov/cgi-bin/browse-edgar?action=getcompany&CIK={}&type=&dateb=&owner=include&count=20&output=atom",
        sym
    );
    let resp = client
        .get(&url)
        .header("User-Agent", "TyphooN Terminal research@typhoon.local")
        .send()
        .await
        .map_err(|e| format!("EDGAR RSS request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("EDGAR RSS: HTTP {}", resp.status()));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| format!("EDGAR RSS read: {e}"))?;
    Ok(parse_atom_items(&body, &sym, "SEC"))
}

/// Marketaux — 100 req/day free, finance-focused, includes sentiment.
pub async fn fetch_marketaux_news(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<NewsArticle>, String> {
    if token.is_empty() {
        return Err("Marketaux API key required".into());
    }
    let sym = symbol.replace("/", "").to_uppercase();
    let resp = client
        .get("https://api.marketaux.com/v1/news/all")
        .query(&[
            ("api_token", token),
            ("symbols", sym.as_str()),
            ("filter_entities", "true"),
            ("language", "en"),
            ("limit", "20"),
        ])
        .send()
        .await
        .map_err(|e| format!("Marketaux request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Marketaux: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Marketaux parse: {e}"))?;
    let mut out = Vec::new();
    if let Some(arr) = v["data"].as_array() {
        for e in arr {
            let url = e["url"].as_str().unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let published_at = parse_iso_ts(e["published_at"].as_str().unwrap_or(""));
            let mut tickers = Vec::new();
            let mut sent_score = 0.0;
            if let Some(ents) = e["entities"].as_array() {
                for en in ents {
                    if let Some(sym) = en["symbol"].as_str() {
                        if !sym.is_empty() {
                            tickers.push(sym.to_uppercase());
                        }
                    }
                    if sent_score == 0.0 {
                        sent_score = en["sentiment_score"].as_f64().unwrap_or(0.0);
                    }
                }
            }
            let sentiment = if sent_score > 0.15 {
                "bullish"
            } else if sent_score < -0.15 {
                "bearish"
            } else {
                "neutral"
            };
            let art = NewsArticle {
                symbol: sym.clone(),
                source: "Marketaux".into(),
                provider: e["source"].as_str().unwrap_or("").to_string(),
                headline: e["title"].as_str().unwrap_or("").to_string(),
                summary: e["description"].as_str().unwrap_or("").to_string(),
                url: url.clone(),
                published_at,
                image_url: e["image_url"].as_str().unwrap_or("").to_string(),
                sentiment: sentiment.into(),
                sentiment_score: sent_score,
                tickers,
                ..Default::default()
            }
            .with_hash();
            out.push(art);
        }
    }
    Ok(out)
}

/// Alpha Vantage NEWS_SENTIMENT — 25 req/day free, sentiment + ticker relevance baked in.
pub async fn fetch_alpha_vantage_news(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<NewsArticle>, String> {
    if token.is_empty() {
        return Err("Alpha Vantage API key required".into());
    }
    let sym = symbol.replace("/", "").to_uppercase();
    let resp = client
        .get("https://www.alphavantage.co/query")
        .query(&[
            ("function", "NEWS_SENTIMENT"),
            ("tickers", sym.as_str()),
            ("limit", "50"),
            ("apikey", token),
        ])
        .send()
        .await
        .map_err(|e| format!("Alpha Vantage request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Alpha Vantage: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("Alpha Vantage parse: {e}"))?;
    if let Some(note) = v["Information"].as_str() {
        if note.contains("limit") || note.contains("rate") {
            return Err(format!("Alpha Vantage rate limit: {note}"));
        }
    }
    let mut out = Vec::new();
    if let Some(arr) = v["feed"].as_array() {
        for e in arr {
            let url = e["url"].as_str().unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let ts_raw = e["time_published"].as_str().unwrap_or("");
            let published_at = parse_av_ts(ts_raw);
            let mut tickers = Vec::new();
            let mut rel_sent = 0.0;
            if let Some(ts_arr) = e["ticker_sentiment"].as_array() {
                for t in ts_arr {
                    if let Some(tsym) = t["ticker"].as_str() {
                        tickers.push(tsym.to_uppercase());
                        if tsym.to_uppercase() == sym {
                            rel_sent = t["ticker_sentiment_score"]
                                .as_str()
                                .and_then(|s| s.parse::<f64>().ok())
                                .unwrap_or(0.0);
                        }
                    }
                }
            }
            let overall_label = e["overall_sentiment_label"].as_str().unwrap_or("");
            let sentiment = match overall_label {
                "Bullish" | "Somewhat-Bullish" => "bullish",
                "Bearish" | "Somewhat-Bearish" => "bearish",
                _ => "neutral",
            };
            let mut categories = Vec::new();
            if let Some(topics) = e["topics"].as_array() {
                for t in topics {
                    if let Some(name) = t["topic"].as_str() {
                        categories.push(name.to_string());
                    }
                }
            }
            let art = NewsArticle {
                symbol: sym.clone(),
                source: "AlphaVantage".into(),
                provider: e["source"].as_str().unwrap_or("").to_string(),
                headline: e["title"].as_str().unwrap_or("").to_string(),
                summary: e["summary"].as_str().unwrap_or("").to_string(),
                url: url.clone(),
                published_at,
                image_url: e["banner_image"].as_str().unwrap_or("").to_string(),
                sentiment: sentiment.into(),
                sentiment_score: rel_sent,
                tickers,
                categories,
                ..Default::default()
            }
            .with_hash();
            out.push(art);
        }
    }
    Ok(out)
}

fn parse_av_ts(s: &str) -> i64 {
    // Format: "20260413T142030"
    if s.len() < 15 {
        return 0;
    }
    let y: i32 = s[0..4].parse().unwrap_or(0);
    let mo: u32 = s[4..6].parse().unwrap_or(0);
    let d: u32 = s[6..8].parse().unwrap_or(0);
    let h: u32 = s[9..11].parse().unwrap_or(0);
    let mi: u32 = s[11..13].parse().unwrap_or(0);
    let sc: u32 = s[13..15].parse().unwrap_or(0);
    chrono::NaiveDate::from_ymd_opt(y, mo, d)
        .and_then(|dt| dt.and_hms_opt(h, mi, sc))
        .map(|ndt| ndt.and_utc().timestamp())
        .unwrap_or(0)
}

/// FMP /v3/stock_news — 250 req/day free, clean normalized shape.
pub async fn fetch_fmp_news(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<NewsArticle>, String> {
    if token.is_empty() {
        return Err("FMP API key required".into());
    }
    let sym = symbol.replace("/", "").to_uppercase();
    let url = format!(
        "https://financialmodelingprep.com/api/v3/stock_news?tickers={}&limit=50&apikey={}",
        sym, token
    );
    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("FMP news request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("FMP news: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("FMP news parse: {e}"))?;
    let mut out = Vec::new();
    for e in arr {
        let url = e["url"].as_str().unwrap_or("").to_string();
        if url.is_empty() {
            continue;
        }
        let published_at = parse_iso_ts(e["publishedDate"].as_str().unwrap_or(""));
        let art = NewsArticle {
            symbol: sym.clone(),
            source: "FMP".into(),
            provider: e["site"].as_str().unwrap_or("").to_string(),
            headline: e["title"].as_str().unwrap_or("").to_string(),
            summary: e["text"].as_str().unwrap_or("").to_string(),
            url: url.clone(),
            published_at,
            image_url: e["image"].as_str().unwrap_or("").to_string(),
            tickers: vec![sym.clone()],
            ..Default::default()
        }
        .with_hash();
        out.push(art);
    }
    Ok(out)
}

pub fn parse_iso_ts(s: &str) -> i64 {
    if s.is_empty() {
        return 0;
    }
    // Handle "2026-04-13T14:20:30Z" or "2026-04-13 14:20:30" or "2026-04-13T14:20:30.000000Z"
    chrono::DateTime::parse_from_rfc3339(s)
        .map(|d| d.timestamp())
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%d %H:%M:%S")
                .map(|ndt| ndt.and_utc().timestamp())
        })
        .or_else(|_| {
            chrono::NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S%.f")
                .map(|ndt| ndt.and_utc().timestamp())
        })
        .unwrap_or(0)
}

// ── RSS / Atom parsing (regex-based, deliberately simple) ────────────────

fn strip_cdata(s: &str) -> String {
    let s = s.trim();
    if let Some(inner) = s
        .strip_prefix("<![CDATA[")
        .and_then(|x| x.strip_suffix("]]>"))
    {
        inner.to_string()
    } else {
        s.to_string()
    }
}

fn decode_entities(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
}

fn extract_tag<'a>(body: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let start = body.find(&open)?;
    let after_open = &body[start..];
    let content_start = after_open.find('>')? + 1;
    let rest = &after_open[content_start..];
    let end = rest.find(&close)?;
    Some(&rest[..end])
}

fn extract_all_tag_bodies<'a>(body: &'a str, tag: &str) -> Vec<&'a str> {
    let open = format!("<{}", tag);
    let close = format!("</{}>", tag);
    let mut out = Vec::new();
    let mut cursor = 0;
    while let Some(rel_start) = body[cursor..].find(&open) {
        let abs_start = cursor + rel_start;
        let after_open = &body[abs_start..];
        let Some(content_offset) = after_open.find('>') else {
            break;
        };
        let content_start_abs = abs_start + content_offset + 1;
        let rest = &body[content_start_abs..];
        let Some(end_rel) = rest.find(&close) else {
            break;
        };
        out.push(&rest[..end_rel]);
        cursor = content_start_abs + end_rel + close.len();
    }
    out
}

fn parse_rss_items(body: &str, symbol: &str, source: &str) -> Vec<NewsArticle> {
    let mut out = Vec::new();
    for item in extract_all_tag_bodies(body, "item") {
        let title = extract_tag(item, "title")
            .map(|s| decode_entities(&strip_cdata(s)))
            .unwrap_or_default();
        let link = extract_tag(item, "link")
            .map(|s| decode_entities(&strip_cdata(s)))
            .unwrap_or_default();
        let desc = extract_tag(item, "description")
            .map(|s| decode_entities(&strip_cdata(s)))
            .unwrap_or_default();
        let pub_date = extract_tag(item, "pubDate")
            .map(|s| strip_cdata(s))
            .unwrap_or_default();
        if link.is_empty() {
            continue;
        }
        let published_at = chrono::DateTime::parse_from_rfc2822(pub_date.trim())
            .map(|d| d.timestamp())
            .unwrap_or(0);
        // Strip inline HTML tags from description.
        let clean_desc = strip_html(&desc);
        let art = NewsArticle {
            symbol: symbol.to_uppercase(),
            source: source.into(),
            provider: String::new(),
            headline: title,
            summary: clean_desc,
            url: link.clone(),
            published_at,
            tickers: vec![symbol.to_uppercase()],
            ..Default::default()
        }
        .with_hash();
        out.push(art);
    }
    out
}

fn parse_atom_items(body: &str, symbol: &str, source: &str) -> Vec<NewsArticle> {
    let mut out = Vec::new();
    for entry in extract_all_tag_bodies(body, "entry") {
        let title = extract_tag(entry, "title")
            .map(|s| decode_entities(&strip_cdata(s)))
            .unwrap_or_default();
        // Atom link is <link href="..."/> — search for href attribute.
        let link = extract_atom_link(entry);
        let summary = extract_tag(entry, "summary")
            .or_else(|| extract_tag(entry, "content"))
            .map(|s| decode_entities(&strip_cdata(s)))
            .unwrap_or_default();
        let updated = extract_tag(entry, "updated")
            .or_else(|| extract_tag(entry, "published"))
            .map(|s| strip_cdata(s))
            .unwrap_or_default();
        if link.is_empty() {
            continue;
        }
        let published_at = chrono::DateTime::parse_from_rfc3339(updated.trim())
            .map(|d| d.timestamp())
            .unwrap_or(0);
        let art = NewsArticle {
            symbol: symbol.to_uppercase(),
            source: source.into(),
            provider: String::new(),
            headline: title,
            summary: strip_html(&summary),
            url: link,
            published_at,
            tickers: vec![symbol.to_uppercase()],
            ..Default::default()
        }
        .with_hash();
        out.push(art);
    }
    out
}

fn extract_atom_link(entry: &str) -> String {
    // <link href="..." /> or <link rel="alternate" href="..."/>
    if let Some(idx) = entry.find("<link") {
        let after = &entry[idx..];
        if let Some(href_idx) = after.find("href=\"") {
            let rest = &after[href_idx + 6..];
            if let Some(end) = rest.find('"') {
                return rest[..end].to_string();
            }
        }
    }
    String::new()
}

fn strip_html(s: &str) -> String {
    // Minimal tag stripper — keeps text, drops <a>, <b>, <img>, etc.
    let mut out = String::with_capacity(s.len());
    let mut in_tag = false;
    for c in s.chars() {
        match c {
            '<' => in_tag = true,
            '>' => in_tag = false,
            _ if !in_tag => out.push(c),
            _ => {}
        }
    }
    decode_entities(&out).trim().to_string()
}

// ── Crypto symbol detection ───────────────────────────────────────────────
//
// Crypto-native news sources (CryptoPanic, CoinDesk, Finnhub `/news?category=crypto`)
// expect base tickers like "BTC" rather than the trading-pair form ("BTC/USD",
// "BTCUSD", "BTC-USD") that users may type. `crypto_base_for_symbol` peels off
// the quote currency and validates against a curated allowlist; anything not on
// the list is treated as a non-crypto symbol so the equity router runs instead.

/// Curated allowlist of crypto base tickers. Used to disambiguate concatenated
/// symbols like "BTCUSD" from equity tickers, and to filter general-feed crypto
/// news (CoinDesk RSS, Finnhub crypto) by base mention.
const CRYPTO_BASES: &[&str] = &[
    "BTC", "ETH", "SOL", "ADA", "DOT", "DOGE", "MATIC", "POL", "AVAX", "LINK", "UNI", "XRP", "LTC",
    "BCH", "ATOM", "ALGO", "NEAR", "FTM", "HBAR", "VET", "SAND", "MANA", "SHIB", "TRX", "ETC",
    "XLM", "USDT", "USDC", "DAI", "WBTC", "FIL", "ICP", "APT", "ARB", "OP", "INJ", "TIA", "SEI",
    "STX", "RNDR", "PYTH", "FET", "TAO", "PEPE", "BONK", "WIF", "FLOKI", "JUP", "STRK", "ENA",
    "ONDO", "SUI", "TON", "MKR", "GRT", "AAVE", "CRV", "SNX", "COMP", "LDO", "RUNE", "KAS", "QNT",
    "XMR", "ZEC", "DASH", "EOS", "NEO", "BAT", "ENJ", "CHZ", "CAKE", "GALA", "AXS", "FLOW", "ROSE",
    "1INCH", "YFI", "BAL", "ZRX", "KSM", "WAVES", "DCR", "OMG", "REN", "STORJ", "ANKR", "CELO",
    "NMR", "RLC", "BAND", "REP", "KAVA", "BNT", "OXT", "GNO", "POLY", "LRC", "NU", "PAXG", "KNC",
    "REQ", "WLD",
];

/// Quote currencies recognised when parsing trading-pair symbols.
const CRYPTO_QUOTES: &[&str] = &[
    "USD", "USDT", "USDC", "DAI", "EUR", "GBP", "JPY", "CHF", "AUD", "CAD", "BTC", "ETH", "XBT",
];

/// Map a base ticker to the full asset name for keyword filtering of general
/// feeds (CoinDesk RSS, Finnhub crypto category). Only the top names with
/// distinctive titles are listed; bases not in this map are filtered by the
/// ticker alone, which is generally fine for shorter names appearing in headlines.
fn crypto_full_name(base: &str) -> Option<&'static str> {
    match base {
        "BTC" | "WBTC" | "XBT" => Some("Bitcoin"),
        "ETH" => Some("Ethereum"),
        "SOL" => Some("Solana"),
        "ADA" => Some("Cardano"),
        "DOT" => Some("Polkadot"),
        "DOGE" => Some("Dogecoin"),
        "MATIC" | "POL" => Some("Polygon"),
        "AVAX" => Some("Avalanche"),
        "LINK" => Some("Chainlink"),
        "UNI" => Some("Uniswap"),
        "XRP" => Some("Ripple"),
        "LTC" => Some("Litecoin"),
        "BCH" => Some("Bitcoin Cash"),
        "ATOM" => Some("Cosmos"),
        "ALGO" => Some("Algorand"),
        "FTM" => Some("Fantom"),
        "HBAR" => Some("Hedera"),
        "SAND" => Some("Sandbox"),
        "MANA" => Some("Decentraland"),
        "SHIB" => Some("Shiba Inu"),
        "TRX" => Some("TRON"),
        "ETC" => Some("Ethereum Classic"),
        "XLM" => Some("Stellar"),
        "USDT" => Some("Tether"),
        "USDC" => Some("USD Coin"),
        "FIL" => Some("Filecoin"),
        "ICP" => Some("Internet Computer"),
        "APT" => Some("Aptos"),
        "ARB" => Some("Arbitrum"),
        "OP" => Some("Optimism"),
        "INJ" => Some("Injective"),
        "TIA" => Some("Celestia"),
        "PYTH" => Some("Pyth"),
        "TAO" => Some("Bittensor"),
        "WLD" => Some("Worldcoin"),
        "JUP" => Some("Jupiter"),
        "STRK" => Some("Starknet"),
        "ONDO" => Some("Ondo"),
        "SUI" => Some("Sui"),
        "TON" => Some("Toncoin"),
        "MKR" => Some("Maker"),
        "GRT" => Some("The Graph"),
        "AAVE" => Some("Aave"),
        "LDO" => Some("Lido"),
        "RUNE" => Some("THORChain"),
        "KAS" => Some("Kaspa"),
        "QNT" => Some("Quant"),
        "XMR" => Some("Monero"),
        _ => None,
    }
}

/// True if `symbol` looks like a crypto pair (BTC/USD, BTCUSD, BTC-USD, BTC).
pub fn is_crypto_symbol(symbol: &str) -> bool {
    crypto_base_for_symbol(symbol).is_some()
}

/// Peel a crypto base ticker out of `symbol`. Recognises:
/// - explicit pair separators: `BTC/USD`, `BTC-USD`
/// - concatenated pairs:       `BTCUSD`, `BTCUSDT`
/// - bare bases:               `BTC`
///
/// Returns `None` if the result isn't in [`CRYPTO_BASES`], so equity tickers
/// like `XOM` (oil) or `BTU` (Peabody) aren't misclassified.
pub fn crypto_base_for_symbol(symbol: &str) -> Option<String> {
    let s = symbol.trim().to_uppercase();
    if s.is_empty() {
        return None;
    }
    // Explicit separators first.
    for sep in ['/', '-', ':'] {
        if let Some((left, right)) = s.split_once(sep) {
            if CRYPTO_BASES.contains(&left) && CRYPTO_QUOTES.contains(&right) {
                return Some(left.to_string());
            }
        }
    }
    // Bare base, e.g. user typed "BTC".
    if CRYPTO_BASES.contains(&s.as_str()) {
        return Some(s);
    }
    // Concatenated form — peel off the longest matching quote suffix.
    for quote in CRYPTO_QUOTES {
        if let Some(base) = s.strip_suffix(quote) {
            if CRYPTO_BASES.contains(&base) {
                return Some(base.to_string());
            }
        }
    }
    None
}

/// True when `headline` or `summary` mentions either the base ticker or the
/// asset's full name. Used to filter general-feed crypto news to articles
/// actually about the requested coin.
fn article_mentions_crypto(headline: &str, summary: &str, base: &str) -> bool {
    let hay = format!("{} {}", headline, summary);
    let hay_upper = hay.to_uppercase();
    if hay_upper.contains(base) {
        return true;
    }
    if let Some(name) = crypto_full_name(base) {
        if hay_upper.contains(&name.to_uppercase()) {
            return true;
        }
    }
    false
}

// ── Crypto-native fetchers ────────────────────────────────────────────────

/// CryptoPanic — public free tier, per-currency filtering.
/// See https://cryptopanic.com/developers/api/ — `auth_token` + `currencies=BTC,ETH`.
pub async fn fetch_cryptopanic_news(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<NewsArticle>, String> {
    if token.is_empty() {
        return Err("CryptoPanic auth token required".into());
    }
    let Some(base) = crypto_base_for_symbol(symbol) else {
        return Ok(vec![]);
    };
    let resp = client
        .get("https://cryptopanic.com/api/v1/posts/")
        .query(&[
            ("auth_token", token),
            ("currencies", base.as_str()),
            ("public", "true"),
            ("kind", "news"),
        ])
        .send()
        .await
        .map_err(|e| format!("CryptoPanic request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("CryptoPanic: HTTP {}", resp.status()));
    }
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("CryptoPanic parse: {e}"))?;
    let mut out = Vec::new();
    if let Some(arr) = v["results"].as_array() {
        for e in arr {
            let url = e["url"].as_str().unwrap_or("").to_string();
            if url.is_empty() {
                continue;
            }
            let published_at = parse_iso_ts(e["published_at"].as_str().unwrap_or(""));
            let mut tickers = Vec::new();
            if let Some(cs) = e["currencies"].as_array() {
                for c in cs {
                    if let Some(code) = c["code"].as_str() {
                        tickers.push(code.to_uppercase());
                    }
                }
            }
            let votes_pos = e["votes"]["positive"].as_i64().unwrap_or(0);
            let votes_neg = e["votes"]["negative"].as_i64().unwrap_or(0);
            let sentiment_score = match (votes_pos, votes_neg) {
                (p, n) if p + n == 0 => 0.0,
                (p, n) => (p - n) as f64 / (p + n) as f64,
            };
            let sentiment = if sentiment_score > 0.15 {
                "bullish"
            } else if sentiment_score < -0.15 {
                "bearish"
            } else {
                "neutral"
            };
            let art = NewsArticle {
                symbol: base.clone(),
                source: "CryptoPanic".into(),
                provider: e["source"]["title"].as_str().unwrap_or("").to_string(),
                headline: e["title"].as_str().unwrap_or("").to_string(),
                summary: String::new(),
                url: url.clone(),
                published_at,
                image_url: String::new(),
                sentiment: sentiment.into(),
                sentiment_score,
                tickers,
                ..Default::default()
            }
            .with_hash();
            out.push(art);
        }
    }
    Ok(out)
}

/// CoinDesk RSS — general crypto news, no key. Filtered to articles mentioning
/// the requested base ticker or its full name.
pub async fn fetch_coindesk_rss(
    client: &reqwest::Client,
    symbol: &str,
) -> Result<Vec<NewsArticle>, String> {
    let Some(base) = crypto_base_for_symbol(symbol) else {
        return Ok(vec![]);
    };
    let resp = client
        .get("https://www.coindesk.com/arc/outboundfeeds/rss/")
        .header(
            "User-Agent",
            "Mozilla/5.0 (X11; Linux x86_64) TyphooN-Terminal/0.1",
        )
        .send()
        .await
        .map_err(|e| format!("CoinDesk RSS request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("CoinDesk RSS: HTTP {}", resp.status()));
    }
    let body = resp
        .text()
        .await
        .map_err(|e| format!("CoinDesk RSS read: {e}"))?;
    let all = parse_rss_items(&body, &base, "CoinDesk");
    let filtered: Vec<NewsArticle> = all
        .into_iter()
        .filter(|a| article_mentions_crypto(&a.headline, &a.summary, &base))
        .collect();
    Ok(filtered)
}

/// Finnhub general crypto feed — same key as `/company-news`, no symbol param.
/// Filtered to articles mentioning the requested base.
pub async fn fetch_finnhub_crypto_news(
    client: &reqwest::Client,
    symbol: &str,
    token: &str,
) -> Result<Vec<NewsArticle>, String> {
    if token.is_empty() {
        return Err("Finnhub key required".into());
    }
    let Some(base) = crypto_base_for_symbol(symbol) else {
        return Ok(vec![]);
    };
    let resp = client
        .get("https://finnhub.io/api/v1/news")
        .query(&[("category", "crypto"), ("token", token)])
        .send()
        .await
        .map_err(|e| format!("Finnhub crypto request failed: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("Finnhub crypto: HTTP {}", resp.status()));
    }
    let arr: Vec<serde_json::Value> = resp
        .json()
        .await
        .map_err(|e| format!("Finnhub crypto parse: {e}"))?;
    let mut out = Vec::new();
    for e in arr {
        let url = e["url"].as_str().unwrap_or("").to_string();
        if url.is_empty() {
            continue;
        }
        let headline = e["headline"].as_str().unwrap_or("").to_string();
        let summary = e["summary"].as_str().unwrap_or("").to_string();
        if !article_mentions_crypto(&headline, &summary, &base) {
            continue;
        }
        let related = e["related"].as_str().unwrap_or("");
        let tickers: Vec<String> = related
            .split(',')
            .filter_map(|s| {
                let t = s.trim().to_uppercase();
                if t.is_empty() { None } else { Some(t) }
            })
            .collect();
        let art = NewsArticle {
            symbol: base.clone(),
            source: "Finnhub".into(),
            provider: e["source"].as_str().unwrap_or("").to_string(),
            headline,
            summary,
            url: url.clone(),
            published_at: e["datetime"].as_i64().unwrap_or(0),
            image_url: e["image"].as_str().unwrap_or("").to_string(),
            tickers,
            ..Default::default()
        }
        .with_hash();
        out.push(art);
    }
    Ok(out)
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
    Ok(all_articles)
}

/// Heuristic — treat alphabetic ticker ≤5 chars as US-listed (EDGAR eligible).
#[cfg(test)]
fn is_us_symbol(s: &str) -> bool {
    !s.is_empty() && s.len() <= 5 && s.chars().all(|c| c.is_ascii_alphabetic() || c == '.')
}

// ── Tests ─────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn mem_conn() -> Connection {
        let conn = Connection::open_in_memory().expect("open in-memory");
        create_news_tables(&conn).expect("create tables");
        conn
    }

    #[test]
    fn hash_is_stable_and_lowercases_url() {
        let a = NewsArticle::compute_hash("https://Example.com/News/Article?Id=1");
        let b = NewsArticle::compute_hash("HTTPS://EXAMPLE.COM/NEWS/ARTICLE?ID=1");
        assert_eq!(a, b);
        assert_eq!(a.len(), 64);
    }

    #[test]
    fn with_hash_populates_hash_field() {
        let a = NewsArticle {
            url: "https://example.com/a".into(),
            ..Default::default()
        }
        .with_hash();
        assert!(!a.url_hash.is_empty());
    }

    #[test]
    fn upsert_and_get_roundtrip() {
        let conn = mem_conn();
        let article = NewsArticle {
            symbol: "AAPL".into(),
            source: "FMP".into(),
            headline: "Apple reports record Q4".into(),
            summary: "AAPL beat estimates...".into(),
            url: "https://example.com/apple-q4".into(),
            published_at: 1_700_000_000,
            sentiment: "bullish".into(),
            sentiment_score: 0.7,
            tickers: vec!["AAPL".into()],
            ..Default::default()
        }
        .with_hash();

        upsert_news(&conn, &article).unwrap();
        let got = get_news_by_symbol(&conn, "AAPL", 10).unwrap();
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].headline, "Apple reports record Q4");
        assert_eq!(got[0].sentiment, "bullish");
    }

    #[test]
    fn upsert_dedup_by_url_hash() {
        let conn = mem_conn();
        let mut a = NewsArticle {
            symbol: "MSFT".into(),
            source: "GDELT".into(),
            headline: "Original headline".into(),
            url: "https://example.com/msft".into(),
            published_at: 1,
            ..Default::default()
        }
        .with_hash();
        upsert_news(&conn, &a).unwrap();

        // Update with new headline, same URL — should merge.
        a.headline = "Updated headline".into();
        upsert_news(&conn, &a).unwrap();

        let rows = get_news_by_symbol(&conn, "MSFT", 10).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].headline, "Updated headline");
    }

    #[test]
    fn fts_search_matches_headline_and_summary() {
        let conn = mem_conn();
        let a = NewsArticle {
            symbol: "TSLA".into(),
            source: "FMP".into(),
            headline: "Tesla beats delivery target".into(),
            summary: "EV maker delivered record number of vehicles.".into(),
            url: "https://example.com/tesla".into(),
            published_at: 1_700_000_000,
            ..Default::default()
        }
        .with_hash();
        upsert_news(&conn, &a).unwrap();

        let hit = search_news(&conn, "delivery", 10).unwrap();
        assert_eq!(hit.len(), 1);

        let hit2 = search_news(&conn, "vehicles", 10).unwrap();
        assert_eq!(hit2.len(), 1);

        let miss = search_news(&conn, "zebra", 10).unwrap();
        assert_eq!(miss.len(), 0);
    }

    #[test]
    fn cached_news_queries_hide_sec_filings() {
        let conn = mem_conn();
        let filing = NewsArticle {
            symbol: "AAPL".into(),
            source: "SEC".into(),
            headline: "10-Q filed".into(),
            summary: "Quarterly report".into(),
            url: "https://sec.gov/aapl-10q".into(),
            published_at: 200,
            ..Default::default()
        }
        .with_hash();
        let story = NewsArticle {
            symbol: "AAPL".into(),
            source: "YahooRSS".into(),
            headline: "Apple rallies on product news".into(),
            summary: "Market story".into(),
            url: "https://example.com/aapl-news".into(),
            published_at: 100,
            ..Default::default()
        }
        .with_hash();
        upsert_news(&conn, &filing).unwrap();
        upsert_news(&conn, &story).unwrap();

        let by_symbol = get_news_by_symbol(&conn, "AAPL", 10).unwrap();
        assert_eq!(by_symbol.len(), 1);
        assert_eq!(by_symbol[0].source, "YahooRSS");

        let all = get_news_by_symbol(&conn, "", 10).unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].source, "YahooRSS");

        let filing_search = search_news(&conn, "Quarterly", 10).unwrap();
        assert!(filing_search.is_empty());
    }

    #[test]
    fn news_scrape_index_gates_repeated_fetches() {
        let conn = mem_conn();
        assert!(!news_cache_is_fresh(&conn, "AAPL", 30 * 60, 1).unwrap());

        let article = NewsArticle {
            symbol: "AAPL".into(),
            source: "YahooRSS".into(),
            headline: "Apple product news".into(),
            url: "https://example.com/aapl-product".into(),
            published_at: 1_700_000_000,
            ..Default::default()
        }
        .with_hash();
        upsert_news(&conn, &article).unwrap();
        assert_eq!(mark_news_scraped(&conn, "AAPL").unwrap(), 1);
        assert!(news_cache_is_fresh(&conn, "aapl", 30 * 60, 1).unwrap());
        assert!(!news_cache_is_fresh(&conn, "aapl", 30 * 60, 2).unwrap());
        let fresh = fresh_news_symbols(&conn, &["aapl".into(), "MSFT".into()], 30 * 60, 1).unwrap();
        assert!(fresh.contains("AAPL"));
        assert!(!fresh.contains("MSFT"));
    }

    #[test]
    fn purge_removes_old_articles() {
        let conn = mem_conn();
        let old = NewsArticle {
            symbol: "A".into(),
            url: "https://example.com/old".into(),
            published_at: 100,
            ..Default::default()
        }
        .with_hash();
        let fresh = NewsArticle {
            symbol: "A".into(),
            url: "https://example.com/new".into(),
            published_at: 999_999_999,
            ..Default::default()
        }
        .with_hash();
        upsert_news(&conn, &old).unwrap();
        upsert_news(&conn, &fresh).unwrap();

        let removed = purge_older_than(&conn, 1000).unwrap();
        assert_eq!(removed, 1);
        let remaining = get_news_by_symbol(&conn, "A", 10).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].url, "https://example.com/new");
    }

    #[test]
    fn parse_gdelt_ts_valid() {
        let t = parse_gdelt_ts("20260413T142030Z");
        assert!(t > 1_700_000_000);
    }

    #[test]
    fn parse_av_ts_valid() {
        let t = parse_av_ts("20260413T142030");
        assert!(t > 1_700_000_000);
    }

    #[test]
    fn parse_iso_ts_variants() {
        assert!(parse_iso_ts("2026-04-13T14:20:30Z") > 0);
        assert!(parse_iso_ts("2026-04-13 14:20:30") > 0);
        assert_eq!(parse_iso_ts(""), 0);
    }

    #[test]
    fn strip_html_removes_tags_and_decodes_entities() {
        let s = strip_html("<a href='x'>Hello</a> &amp; <b>world</b>");
        assert_eq!(s, "Hello & world");
    }

    #[test]
    fn rss_item_parser_extracts_fields() {
        let rss = r#"
        <rss><channel>
            <item>
                <title><![CDATA[Apple rallies on earnings]]></title>
                <link>https://example.com/apple</link>
                <description>Apple beat expectations...</description>
                <pubDate>Mon, 13 Apr 2026 14:20:30 GMT</pubDate>
            </item>
            <item>
                <title>Microsoft cloud growth</title>
                <link>https://example.com/msft</link>
                <description>Azure posted 30% growth</description>
                <pubDate>Mon, 13 Apr 2026 10:00:00 GMT</pubDate>
            </item>
        </channel></rss>
        "#;
        let items = parse_rss_items(rss, "AAPL", "YahooRSS");
        assert_eq!(items.len(), 2);
        assert_eq!(items[0].headline, "Apple rallies on earnings");
        assert_eq!(items[0].url, "https://example.com/apple");
        assert_eq!(items[1].headline, "Microsoft cloud growth");
    }

    #[test]
    fn atom_parser_extracts_link_from_href() {
        let atom = r#"
        <feed>
            <entry>
                <title>10-Q filed</title>
                <link href="https://sec.gov/a.htm" rel="alternate"/>
                <summary>Quarterly report</summary>
                <updated>2026-04-13T14:20:30Z</updated>
            </entry>
        </feed>
        "#;
        let items = parse_atom_items(atom, "AAPL", "SEC");
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].url, "https://sec.gov/a.htm");
        assert_eq!(items[0].headline, "10-Q filed");
    }

    #[test]
    fn is_us_symbol_heuristic() {
        assert!(is_us_symbol("AAPL"));
        assert!(is_us_symbol("T"));
        assert!(is_us_symbol("BRK.A"));
        assert!(!is_us_symbol(""));
        assert!(!is_us_symbol("EURUSD"));
        assert!(!is_us_symbol("BTC/USD"));
    }

    #[test]
    fn crypto_base_for_symbol_recognises_pair_forms() {
        assert_eq!(crypto_base_for_symbol("BTC/USD").as_deref(), Some("BTC"));
        assert_eq!(crypto_base_for_symbol("eth-usd").as_deref(), Some("ETH"));
        assert_eq!(crypto_base_for_symbol("SOLUSDT").as_deref(), Some("SOL"));
        assert_eq!(crypto_base_for_symbol("BTC").as_deref(), Some("BTC"));
        // lowercase still works
        assert_eq!(crypto_base_for_symbol("doge/usd").as_deref(), Some("DOGE"));
    }

    #[test]
    fn crypto_base_for_symbol_rejects_equities() {
        // Equity tickers that happen to overlap a coin format must not match.
        assert!(crypto_base_for_symbol("AAPL").is_none());
        assert!(crypto_base_for_symbol("SPY").is_none());
        assert!(crypto_base_for_symbol("BRK.A").is_none());
        assert!(crypto_base_for_symbol("").is_none());
    }

    #[test]
    fn article_mentions_crypto_matches_ticker_or_name() {
        assert!(article_mentions_crypto("BTC pumps 5%", "", "BTC"));
        assert!(article_mentions_crypto(
            "Bitcoin hits new ATH",
            "spot inflows surge",
            "BTC"
        ));
        assert!(!article_mentions_crypto("Apple beats earnings", "", "BTC"));
    }

    #[test]
    fn get_news_empty_symbol_returns_all() {
        let conn = mem_conn();
        let a1 = NewsArticle {
            symbol: "A".into(),
            url: "https://example.com/1".into(),
            published_at: 100,
            headline: "h1".into(),
            ..Default::default()
        }
        .with_hash();
        let a2 = NewsArticle {
            symbol: "B".into(),
            url: "https://example.com/2".into(),
            published_at: 200,
            headline: "h2".into(),
            ..Default::default()
        }
        .with_hash();
        upsert_news(&conn, &a1).unwrap();
        upsert_news(&conn, &a2).unwrap();
        let all = get_news_by_symbol(&conn, "", 10).unwrap();
        assert_eq!(all.len(), 2);
        // Descending by published_at
        assert_eq!(all[0].headline, "h2");
    }

    #[test]
    fn upsert_batch_counts_rows() {
        let conn = mem_conn();
        let articles: Vec<NewsArticle> = (0..5)
            .map(|i| {
                NewsArticle {
                    symbol: "X".into(),
                    url: format!("https://example.com/{i}"),
                    published_at: 1000 + i,
                    ..Default::default()
                }
                .with_hash()
            })
            .collect();
        let n = upsert_news_batch(&conn, &articles).unwrap();
        assert_eq!(n, 5);
    }
}
