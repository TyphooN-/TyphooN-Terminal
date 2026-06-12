use super::*;

// ── Web article ingestion (JSON-blob-per-symbol, schema v23) ──
//
// Agent-supplied web research articles. When the research packet's
// "Return Path" footer asks an AI agent to emit a fenced
// `===TYPHOON_INGEST===` block of article objects, the INGEST_RESEARCH
// command parses that block and merges the articles into the
// `research_web_articles` cache. LAN sync then distributes the
// ingested corpus to peer terminals.

/// One web research article captured from an AI agent's reply.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WebArticle {
    pub title: String,
    pub url: String,
    pub source: String,       // publication / domain
    pub published_at: String, // ISO-8601 preferred, any string tolerated
    pub summary: String,
    pub agent_used: String, // "claude" | "gemini" | "chatgpt" | free-form
    pub ingested_at: i64,   // unix seconds
    /// Full article text when the agent actually fetched the source (web_search /
    /// browse tools). Empty when the agent only had access to the headline / their
    /// own synthesis — in that case `summary` is the only content. `serde` default
    /// keeps this field backward-compatible with pre-existing
    /// `research_web_articles` JSON blobs and with LAN peers running older builds.
    #[serde(default)]
    pub body: String,
}

/// Per-symbol bag of ingested web articles. JSON-blob-per-symbol.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IngestedArticlesSnapshot {
    pub symbol: String,
    pub articles: Vec<WebArticle>,
}

/// Max articles retained per symbol (FIFO drop by ingested_at).
pub const INGESTED_ARTICLES_MAX: usize = 50;

pub fn create_research_tables_v23(conn: &Connection) -> Result<(), String> {
    let _ = create_research_tables_v22(conn);
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS research_web_articles (
            symbol TEXT PRIMARY KEY,
            snapshot_json TEXT NOT NULL DEFAULT '{}',
            updated_at INTEGER NOT NULL DEFAULT 0
        );
        CREATE INDEX IF NOT EXISTS idx_research_web_articles_updated ON research_web_articles(updated_at);",
    ).map_err(|e| format!("create v23 tables: {e}"))?;
    Ok(())
}

pub fn upsert_ingested_articles(
    conn: &Connection,
    symbol: &str,
    snap: &IngestedArticlesSnapshot,
) -> Result<(), String> {
    let _ = create_research_tables_v23(conn);
    let json = serde_json::to_string(snap).map_err(|e| format!("ingested articles json: {e}"))?;
    conn.execute(
        "INSERT INTO research_web_articles(symbol, snapshot_json, updated_at) VALUES (?1,?2,?3)
         ON CONFLICT(symbol) DO UPDATE SET snapshot_json=excluded.snapshot_json, updated_at=excluded.updated_at",
        params![symbol.to_uppercase(), json, now_ts()],
    ).map_err(|e| format!("upsert ingested articles: {e}"))?;
    Ok(())
}

pub fn get_ingested_articles(
    conn: &Connection,
    symbol: &str,
) -> Result<Option<IngestedArticlesSnapshot>, String> {
    let _ = create_research_tables_v23(conn);
    let mut stmt = conn
        .prepare("SELECT snapshot_json FROM research_web_articles WHERE symbol = ?1")
        .map_err(|e| format!("prepare get_ingested_articles: {e}"))?;
    let mut r = stmt
        .query(params![symbol.to_uppercase()])
        .map_err(|e| format!("query get_ingested_articles: {e}"))?;
    if let Some(row) = r
        .next()
        .map_err(|e| format!("row get_ingested_articles: {e}"))?
    {
        let json: String = row.get(0).unwrap_or_default();
        Ok(Some(serde_json::from_str(&json).unwrap_or_default()))
    } else {
        Ok(None)
    }
}

/// Merge new articles into the symbol's existing bag.
///
/// Dedupe by URL (case-insensitive). On conflict the newer entry wins
/// (articles with a larger `ingested_at` replace older ones). After
/// merging, the bag is trimmed to the latest `INGESTED_ARTICLES_MAX`
/// articles by `ingested_at` (most-recent first, FIFO drop of oldest).
/// Returns `(added_count, total_count)`.
pub fn append_ingested_articles(
    conn: &Connection,
    symbol: &str,
    incoming: Vec<WebArticle>,
) -> Result<(usize, usize), String> {
    let _ = create_research_tables_v23(conn);
    let mut existing =
        get_ingested_articles(conn, symbol)?.unwrap_or_else(|| IngestedArticlesSnapshot {
            symbol: symbol.to_uppercase(),
            articles: Vec::new(),
        });

    let before = existing.articles.len();
    let mut by_url: std::collections::HashMap<String, usize> = existing
        .articles
        .iter()
        .enumerate()
        .map(|(idx, article)| (article.url.trim().to_ascii_lowercase(), idx))
        .collect();

    for mut art in incoming {
        if art.url.trim().is_empty() {
            continue;
        }
        if art.ingested_at == 0 {
            art.ingested_at = now_ts();
        }
        let key = art.url.trim().to_ascii_lowercase();
        if let Some(&pos) = by_url.get(&key) {
            if art.ingested_at >= existing.articles[pos].ingested_at {
                existing.articles[pos] = art;
            }
        } else {
            let pos = existing.articles.len();
            existing.articles.push(art);
            by_url.insert(key, pos);
        }
    }

    existing
        .articles
        .sort_by(|a, b| b.ingested_at.cmp(&a.ingested_at));
    if existing.articles.len() > INGESTED_ARTICLES_MAX {
        existing.articles.truncate(INGESTED_ARTICLES_MAX);
    }
    let after = existing.articles.len();
    let added = after.saturating_sub(before);

    upsert_ingested_articles(conn, symbol, &existing)?;
    Ok((added, after))
}

/// Parse one or more fenced `===TYPHOON_INGEST===` blocks out of an
/// AI agent reply and return them grouped by uppercase symbol.
///
/// Block format (the footer appended to research packets asks agents
/// to emit exactly this):
///
/// ```text
/// ===TYPHOON_INGEST===
/// [
///   {"symbol": "AAPL", "title": "...", "url": "...", "source": "...",
///    "published_at": "2026-04-15", "summary": "...", "agent": "claude"},
///   ...
/// ]
/// ===END_INGEST===
/// ```
///
/// The parser is lenient: it accepts `published` / `date` as aliases
/// for `published_at`, `agent` for `agent_used`, and silently skips
/// entries with no `url` or no `symbol`. It also tolerates surrounding
/// ```json fences and surrounding whitespace. The `ingested_at` field
/// is always set to the current timestamp at parse time.
pub fn parse_ingest_block(text: &str) -> Vec<(String, Vec<WebArticle>)> {
    let mut out: std::collections::BTreeMap<String, Vec<WebArticle>> =
        std::collections::BTreeMap::new();
    let now = now_ts();

    let mut rest = text;
    loop {
        let start = match rest.find("===TYPHOON_INGEST===") {
            Some(i) => i,
            None => break,
        };
        let after_start = &rest[start + "===TYPHOON_INGEST===".len()..];
        let end_idx = match after_start.find("===END_INGEST===") {
            Some(i) => i,
            None => after_start.len(),
        };
        let mut block = after_start[..end_idx].trim().to_string();

        // Strip ```json / ``` fences if present.
        if block.starts_with("```") {
            if let Some(nl) = block.find('\n') {
                block = block[nl + 1..].to_string();
            }
        }
        if block.ends_with("```") {
            let cut = block.len() - 3;
            block = block[..cut].trim_end().to_string();
        }

        if let Ok(v) = serde_json::from_str::<serde_json::Value>(&block) {
            if let Some(arr) = v.as_array() {
                for item in arr {
                    let obj = match item.as_object() {
                        Some(o) => o,
                        None => continue,
                    };
                    let symbol = obj
                        .get("symbol")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_uppercase();
                    if symbol.is_empty() {
                        continue;
                    }
                    let url = obj
                        .get("url")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .trim()
                        .to_string();
                    if url.is_empty() {
                        continue;
                    }
                    let title = obj
                        .get("title")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    let source = obj
                        .get("source")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    let published_at = obj
                        .get("published_at")
                        .and_then(|s| s.as_str())
                        .or_else(|| obj.get("published").and_then(|s| s.as_str()))
                        .or_else(|| obj.get("date").and_then(|s| s.as_str()))
                        .unwrap_or("")
                        .to_string();
                    let summary = obj
                        .get("summary")
                        .and_then(|s| s.as_str())
                        .unwrap_or("")
                        .to_string();
                    let agent_used = obj
                        .get("agent_used")
                        .and_then(|s| s.as_str())
                        .or_else(|| obj.get("agent").and_then(|s| s.as_str()))
                        .unwrap_or("")
                        .to_string();
                    // Optional `body` field: full article text when the agent
                    // actually fetched the source. Accepts `body` or `text` as
                    // aliases since different LLMs default to different keys.
                    let body = obj
                        .get("body")
                        .and_then(|s| s.as_str())
                        .or_else(|| obj.get("text").and_then(|s| s.as_str()))
                        .unwrap_or("")
                        .to_string();
                    out.entry(symbol).or_default().push(WebArticle {
                        title,
                        url,
                        source,
                        published_at,
                        summary,
                        agent_used,
                        ingested_at: now,
                        body,
                    });
                }
            }
        }

        rest = &after_start[end_idx..];
        if rest.is_empty() {
            break;
        }
        if let Some(skip) = rest.find("===END_INGEST===") {
            rest = &rest[skip + "===END_INGEST===".len()..];
        } else {
            break;
        }
    }

    out.into_iter().collect()
}
