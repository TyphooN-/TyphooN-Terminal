//! News ingest — multi-source aggregation with SQLite cache and FTS5 search.
//!
//! Sources (all free tier, most without API keys):
//! - **GDELT 2.0 Doc API** — no key, global coverage, JSON (`https://api.gdeltproject.org/api/v2/doc/doc`)
//! - **Yahoo Finance RSS** — no key, per-symbol (`https://feeds.finance.yahoo.com/rss/2.0/headline?s=SYM`)
//! - **Marketaux** — 100 req/day free, finance-focused, sentiment tags
//! - **Alpha Vantage NEWS_SENTIMENT** — 25 req/day free, built-in sentiment + tickers
//! - **FMP /v3/stock_news** — 250 req/day free, clean normalized format
//!
//! All fetchers normalize into `NewsArticle` and upsert into `research_news` keyed by
//! SHA-256 of the canonical URL so the same story from two sources collapses to one row.
//!
//! The `research_news_fts` FTS5 virtual table mirrors headline + summary so the NEWS
//! window can do keyword search across cached articles instantly.

use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

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
}

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
            tokenize='porter unicode61'
        );",
    )
    .map_err(|e| format!("create news tables: {e}"))?;
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
          image_url, sentiment, sentiment_score, tickers_json, categories_json, updated_at)
         VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14)
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
            updated_at = excluded.updated_at",
        params![
            a.url_hash, a.symbol.to_uppercase(), a.source, a.provider, a.headline, a.summary,
            a.url, a.published_at, a.image_url, a.sentiment, a.sentiment_score,
            tickers_json, categories_json, now_ts(),
        ],
    ).map_err(|e| format!("upsert news: {e}"))?;

    // FTS5 mirror — DELETE then INSERT for upsert semantics.
    let _ = conn.execute(
        "DELETE FROM research_news_fts WHERE url_hash = ?1",
        params![a.url_hash],
    );
    let _ = conn.execute(
        "INSERT INTO research_news_fts(url_hash, headline, summary) VALUES (?1,?2,?3)",
        params![a.url_hash, a.headline, a.summary],
    );
    Ok(())
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
                image_url, sentiment, sentiment_score, tickers_json, categories_json
         FROM research_news
         WHERE source <> 'SEC'
         ORDER BY published_at DESC, updated_at DESC
         LIMIT ?1"
    } else {
        "SELECT url_hash, symbol, source, provider, headline, summary, url, published_at,
                image_url, sentiment, sentiment_score, tickers_json, categories_json
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
                n.image_url, n.sentiment, n.sentiment_score, n.tickers_json, n.categories_json
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

// ── Bulk scrape helper ────────────────────────────────────────────────────

/// Fetch news for a symbol from all configured sources.
/// Sources without keys are skipped silently. Rate-limits with sleeps between calls.
///
/// This function is pure-async with no SQLite dependency — callers must upsert
/// the returned vector themselves via `upsert_news_batch`. Splitting fetch from
/// persistence lets us call this safely from a tokio task without dragging a
/// non-Send `&rusqlite::Connection` across await boundaries.
pub async fn fetch_all_sources_for_symbol(
    client: &reqwest::Client,
    symbol: &str,
    marketaux_key: &str,
    alpha_vantage_key: &str,
    fmp_key: &str,
    mut cb: impl FnMut(&str),
) -> Result<Vec<NewsArticle>, String> {
    let sym = symbol.to_uppercase();
    if sym.is_empty() {
        return Err("empty symbol".into());
    }

    let mut all_articles: Vec<NewsArticle> = Vec::new();

    // GDELT (no key)
    match fetch_gdelt_news(client, &sym, 30).await {
        Ok(v) => {
            cb(&format!("news/gdelt {}: {} articles", sym, v.len()));
            all_articles.extend(v);
        }
        Err(e) => cb(&format!("news/gdelt {} failed: {e}", sym)),
    }
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;

    // Yahoo RSS (no key)
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

    // Marketaux (100/day free)
    if !marketaux_key.is_empty() {
        match fetch_marketaux_news(client, &sym, marketaux_key).await {
            Ok(v) => {
                cb(&format!("news/marketaux {}: {} articles", sym, v.len()));
                all_articles.extend(v);
            }
            Err(e) => cb(&format!("news/marketaux {} failed: {e}", sym)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(900)).await;
    }

    // Alpha Vantage (25/day free)
    if !alpha_vantage_key.is_empty() {
        match fetch_alpha_vantage_news(client, &sym, alpha_vantage_key).await {
            Ok(v) => {
                cb(&format!("news/alpha_vantage {}: {} articles", sym, v.len()));
                all_articles.extend(v);
            }
            Err(e) => cb(&format!("news/alpha_vantage {} failed: {e}", sym)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(1200)).await;
    }

    // FMP (250/day free)
    if !fmp_key.is_empty() {
        match fetch_fmp_news(client, &sym, fmp_key).await {
            Ok(v) => {
                cb(&format!("news/fmp {}: {} articles", sym, v.len()));
                all_articles.extend(v);
            }
            Err(e) => cb(&format!("news/fmp {} failed: {e}", sym)),
        }
        tokio::time::sleep(std::time::Duration::from_millis(300)).await;
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
