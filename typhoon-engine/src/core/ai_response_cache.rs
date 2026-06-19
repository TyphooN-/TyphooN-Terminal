//! Cross-client AI response cache.
//!
//! Deduplicates AI-provider calls by hashing the normalised prompt tuple
//! (provider, model, system, history, message) and checking the cache before
//! spending tokens on another call.
//!
//! Storage: a regular `ai_response_cache` table (NOT the `kv_cache` KV store),
//! so one expensive hosted-model answer can satisfy identical prompts from
//! the native app and CLI.
//!
//! Invalidation: primary invalidation is automatic — the research packet
//! changes every day (new prices, new fundamentals), which changes the hash,
//! which forces a cache miss. The `created_at` column plus a soft TTL lets
//! callers prune very old entries; there is no hard expiry in SQL itself.
//!
//! Privacy: the cache stores normalised prompts in cleartext for debugging
//! and inspection.
//!
//! See for the full design.

use crate::core::cache::SqliteCache as Cache;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// One cached AI response.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiResponseCacheEntry {
    pub prompt_hash: String,     // hex sha256 over the normalised prompt tuple
    pub provider: String, // "claude_http" | "openai" | "gemini" | "grok" | "mistral" | "perplexity" | "local" | "claude_cli" | "gemini_cli" | "codex_cli"
    pub model: String,    // provider-specific model id at call time
    pub prompt_preview: String, // last-user-message trimmed to ~400 chars for the stats window
    pub response: String, // the assistant reply in full
    pub token_count_prompt: i64, // best-effort estimate; 0 if unknown
    pub token_count_completion: i64, // best-effort estimate; 0 if unknown
    pub created_at: i64,  // unix seconds — when the entry was first inserted
    pub updated_at: i64,  // unix seconds — refreshed on each cache hit
    pub hit_count: i64,   // incremented on every cache-hit; the original insertion counts as 0
    pub source_client: String, // hostname of the client that originated this entry; empty if unknown
}

/// Canonicalise a prompt tuple and produce a deterministic hash.
///
/// The hash covers every input that can change the model output:
/// provider, model, system prompt, prior turns, the new user message, and a
/// coarse packet-size bucket (so near-identical packets with minor noise still
/// collide while entirely different packets do not). Whitespace is trimmed,
/// internal newlines are kept as-is (they carry semantics in research packets).
pub fn hash_ai_prompt(
    provider: &str,
    model: &str,
    system: &str,
    history: &[(bool, String)],
    message: &str,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"v1\0");
    hasher.update(provider.trim().as_bytes());
    hasher.update(b"\0");
    hasher.update(model.trim().as_bytes());
    hasher.update(b"\0");
    hasher.update(system.trim().as_bytes());
    hasher.update(b"\0");
    for (is_user, text) in history.iter() {
        hasher.update(if *is_user { b"u\0" } else { b"a\0" });
        hasher.update(text.trim().as_bytes());
        hasher.update(b"\0");
    }
    hasher.update(b"msg\0");
    hasher.update(message.trim().as_bytes());
    let out = hasher.finalize();
    hex_encode(&out)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

/// Create the table if it doesn't yet exist. Idempotent.
pub fn create_ai_response_cache_table(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS ai_response_cache (
            prompt_hash TEXT PRIMARY KEY,
            provider TEXT NOT NULL,
            model TEXT NOT NULL,
            prompt_preview TEXT NOT NULL DEFAULT '',
            response TEXT NOT NULL,
            token_count_prompt INTEGER NOT NULL DEFAULT 0,
            token_count_completion INTEGER NOT NULL DEFAULT 0,
            created_at INTEGER NOT NULL,
            updated_at INTEGER NOT NULL,
            hit_count INTEGER NOT NULL DEFAULT 0,
            source_client TEXT NOT NULL DEFAULT ''
         );
         CREATE INDEX IF NOT EXISTS idx_ai_response_cache_updated
            ON ai_response_cache (updated_at DESC);
         CREATE INDEX IF NOT EXISTS idx_ai_response_cache_provider_model
            ON ai_response_cache (provider, model);",
    )
    .map_err(|e| format!("create ai_response_cache: {e}"))
}

/// Insert a fresh response into the cache.
///
/// `source_client` should be the originating client hostname; empty is fine
/// for local-only testing. If an entry for the same `prompt_hash` already
/// exists it is replaced — this lets later (presumably better-token-counted)
/// insertions supersede earlier ones.
pub fn upsert_response(cache: &Cache, entry: &AiResponseCacheEntry) -> Result<(), String> {
    let conn = cache.connection()?;
    create_ai_response_cache_table(&conn)?;
    let now = now_ts();
    let created_at = if entry.created_at > 0 {
        entry.created_at
    } else {
        now
    };
    conn.execute(
        "INSERT INTO ai_response_cache
            (prompt_hash, provider, model, prompt_preview, response,
             token_count_prompt, token_count_completion,
             created_at, updated_at, hit_count, source_client)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
         ON CONFLICT(prompt_hash) DO UPDATE SET
            provider = excluded.provider,
            model = excluded.model,
            prompt_preview = excluded.prompt_preview,
            response = excluded.response,
            token_count_prompt = excluded.token_count_prompt,
            token_count_completion = excluded.token_count_completion,
            updated_at = excluded.updated_at,
            source_client = excluded.source_client",
        params![
            entry.prompt_hash,
            entry.provider,
            entry.model,
            entry.prompt_preview,
            entry.response,
            entry.token_count_prompt,
            entry.token_count_completion,
            created_at,
            now,
            entry.hit_count,
            entry.source_client,
        ],
    )
    .map_err(|e| format!("upsert ai_response_cache: {e}"))?;
    Ok(())
}

/// Look up a cached response by prompt hash. Returns None on miss.
///
/// On hit, increments `hit_count` and refreshes `updated_at` so the timestamp
/// column reflects recent use (rather than cold storage).
pub fn lookup_response(
    cache: &Cache,
    prompt_hash: &str,
) -> Result<Option<AiResponseCacheEntry>, String> {
    let conn = cache.connection()?;
    create_ai_response_cache_table(&conn)?;
    let row: Option<AiResponseCacheEntry> = conn
        .query_row(
            "SELECT prompt_hash, provider, model, prompt_preview, response,
                token_count_prompt, token_count_completion,
                created_at, updated_at, hit_count, source_client
         FROM ai_response_cache WHERE prompt_hash = ?1",
            params![prompt_hash],
            |r| {
                Ok(AiResponseCacheEntry {
                    prompt_hash: r.get(0)?,
                    provider: r.get(1)?,
                    model: r.get(2)?,
                    prompt_preview: r.get(3)?,
                    response: r.get(4)?,
                    token_count_prompt: r.get(5)?,
                    token_count_completion: r.get(6)?,
                    created_at: r.get(7)?,
                    updated_at: r.get(8)?,
                    hit_count: r.get(9)?,
                    source_client: r.get(10)?,
                })
            },
        )
        .optional()
        .map_err(|e| format!("lookup ai_response_cache: {e}"))?;

    if let Some(ref _hit) = row {
        let now = now_ts();
        let _ = conn.execute(
            "UPDATE ai_response_cache
             SET hit_count = hit_count + 1, updated_at = ?1
             WHERE prompt_hash = ?2",
            params![now, prompt_hash],
        );
    }
    Ok(row.map(|mut e| {
        // Reflect the in-memory delta so callers don't need to re-query.
        e.hit_count += 1;
        e.updated_at = now_ts();
        e
    }))
}

/// Aggregate statistics for the AICACHE window.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiResponseCacheStats {
    pub entry_count: i64,
    pub total_hits: i64,               // sum of hit_count over all entries
    pub tokens_saved_prompt: i64,      // sum(token_count_prompt * hit_count)
    pub tokens_saved_completion: i64,  // sum(token_count_completion * hit_count)
    pub oldest_created_at: i64,        // 0 when empty
    pub newest_updated_at: i64,        // 0 when empty
    pub providers: Vec<(String, i64)>, // (provider, entry_count) sorted desc by count
}

/// Read aggregate stats across all entries. Cheap — single table scan.
pub fn stats(cache: &Cache) -> Result<AiResponseCacheStats, String> {
    let conn = cache.connection()?;
    create_ai_response_cache_table(&conn)?;
    let mut out = AiResponseCacheStats::default();

    let _ = conn.query_row(
        "SELECT COUNT(*),
                COALESCE(SUM(hit_count), 0),
                COALESCE(SUM(token_count_prompt * hit_count), 0),
                COALESCE(SUM(token_count_completion * hit_count), 0),
                COALESCE(MIN(created_at), 0),
                COALESCE(MAX(updated_at), 0)
         FROM ai_response_cache",
        [],
        |r| {
            out.entry_count = r.get(0)?;
            out.total_hits = r.get(1)?;
            out.tokens_saved_prompt = r.get(2)?;
            out.tokens_saved_completion = r.get(3)?;
            out.oldest_created_at = r.get(4)?;
            out.newest_updated_at = r.get(5)?;
            Ok(())
        },
    );

    let mut stmt = conn
        .prepare(
            "SELECT provider, COUNT(*) c FROM ai_response_cache GROUP BY provider ORDER BY c DESC",
        )
        .map_err(|e| format!("prepare stats provider agg: {e}"))?;
    let rows = stmt
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)))
        .map_err(|e| format!("run stats provider agg: {e}"))?;
    for row in rows {
        if let Ok((p, c)) = row {
            out.providers.push((p, c));
        }
    }
    Ok(out)
}

/// List the most-recently-updated entries, most recent first. Used by the
/// AICACHE window to show recent activity.
pub fn recent_entries(cache: &Cache, limit: usize) -> Result<Vec<AiResponseCacheEntry>, String> {
    let conn = cache.connection()?;
    create_ai_response_cache_table(&conn)?;
    let limit_i = limit as i64;
    let mut stmt = conn
        .prepare(
            "SELECT prompt_hash, provider, model, prompt_preview, response,
                token_count_prompt, token_count_completion,
                created_at, updated_at, hit_count, source_client
         FROM ai_response_cache
         ORDER BY updated_at DESC
         LIMIT ?1",
        )
        .map_err(|e| format!("prepare recent_entries: {e}"))?;
    let rows = stmt
        .query_map(params![limit_i], |r| {
            Ok(AiResponseCacheEntry {
                prompt_hash: r.get(0)?,
                provider: r.get(1)?,
                model: r.get(2)?,
                prompt_preview: r.get(3)?,
                response: r.get(4)?,
                token_count_prompt: r.get(5)?,
                token_count_completion: r.get(6)?,
                created_at: r.get(7)?,
                updated_at: r.get(8)?,
                hit_count: r.get(9)?,
                source_client: r.get(10)?,
            })
        })
        .map_err(|e| format!("run recent_entries: {e}"))?;

    let mut out = Vec::new();
    for row in rows {
        if let Ok(e) = row {
            out.push(e);
        }
    }
    Ok(out)
}

/// Coarse token estimate — 1 token per 4 chars, which is the commonly-cited
/// rule of thumb for English text in GPT-family tokenisers. Good enough for
/// the "tokens saved" display in the AICACHE window; exact accounting would
/// require provider-specific tokenizers which we don't ship.
pub fn estimate_tokens(text: &str) -> i64 {
    let c = text.chars().count();
    ((c + 3) / 4) as i64
}

/// Prune entries older than `max_age_secs` measured from `created_at`. Returns
/// the number of rows deleted. Callers decide the policy; `0` means never
/// prune.
pub fn prune_older_than(cache: &Cache, max_age_secs: i64) -> Result<usize, String> {
    if max_age_secs <= 0 {
        return Ok(0);
    }
    let conn = cache.connection()?;
    create_ai_response_cache_table(&conn)?;
    let cutoff = now_ts() - max_age_secs;
    let n = conn
        .execute(
            "DELETE FROM ai_response_cache WHERE created_at < ?1",
            params![cutoff],
        )
        .map_err(|e| format!("prune ai_response_cache: {e}"))?;
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::cache::SqliteCache;
    use std::path::PathBuf;

    fn tmp_cache() -> SqliteCache {
        let p = PathBuf::from(format!(
            "/tmp/ai_response_cache_test_{}.db",
            std::process::id() as u64 * 100 + rand_suffix()
        ));
        let _ = std::fs::remove_file(&p);
        SqliteCache::open(&p).expect("open cache")
    }

    fn rand_suffix() -> u64 {
        use std::time::{SystemTime, UNIX_EPOCH};
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.subsec_nanos() as u64)
            .unwrap_or(0)
            % 10_000
    }

    fn set_updated_at_for_test(cache: &SqliteCache, prompt_hash: &str, updated_at: i64) {
        let conn = cache.connection().unwrap();
        conn.execute(
            "UPDATE ai_response_cache SET updated_at = ?1 WHERE prompt_hash = ?2",
            params![updated_at, prompt_hash],
        )
        .unwrap();
    }

    #[test]
    fn hash_is_deterministic() {
        let h1 = hash_ai_prompt("claude_http", "claude-opus-4-5", "sys", &[], "hi");
        let h2 = hash_ai_prompt("claude_http", "claude-opus-4-5", "sys", &[], "hi");
        assert_eq!(h1, h2);
        assert_eq!(h1.len(), 64, "sha256 hex should be 64 chars");
    }

    #[test]
    fn hash_changes_on_any_input_change() {
        let h = hash_ai_prompt("claude_http", "claude-opus-4-5", "sys", &[], "hi");
        assert_ne!(
            h,
            hash_ai_prompt("openai", "claude-opus-4-5", "sys", &[], "hi"),
            "provider"
        );
        assert_ne!(
            h,
            hash_ai_prompt("claude_http", "claude-sonnet-4-6", "sys", &[], "hi"),
            "model"
        );
        assert_ne!(
            h,
            hash_ai_prompt("claude_http", "claude-opus-4-5", "sys2", &[], "hi"),
            "system"
        );
        assert_ne!(
            h,
            hash_ai_prompt("claude_http", "claude-opus-4-5", "sys", &[], "bye"),
            "message"
        );
        assert_ne!(
            h,
            hash_ai_prompt(
                "claude_http",
                "claude-opus-4-5",
                "sys",
                &[(true, "prev".into())],
                "hi"
            ),
            "history"
        );
    }

    #[test]
    fn hash_ignores_leading_trailing_whitespace() {
        let h1 = hash_ai_prompt("claude_http", "m", "sys", &[], "hi");
        let h2 = hash_ai_prompt("  claude_http\n", "m", " sys  ", &[], "  hi\n");
        assert_eq!(h1, h2);
    }

    #[test]
    fn roundtrip_insert_and_lookup() {
        let cache = tmp_cache();
        let entry = AiResponseCacheEntry {
            prompt_hash: "abc123".into(),
            provider: "claude_http".into(),
            model: "claude-opus-4-5".into(),
            prompt_preview: "What is the P/E of AAPL?".into(),
            response: "AAPL trades at ~29x forward earnings...".into(),
            token_count_prompt: 2500,
            token_count_completion: 400,
            created_at: 0,
            updated_at: 0,
            hit_count: 0,
            source_client: "laptop-1".into(),
        };
        upsert_response(&cache, &entry).unwrap();
        let hit = lookup_response(&cache, "abc123").unwrap().expect("hit");
        assert_eq!(hit.response, entry.response);
        assert_eq!(hit.hit_count, 1, "lookup should increment hit_count to 1");
    }

    #[test]
    fn lookup_miss_returns_none() {
        let cache = tmp_cache();
        assert!(lookup_response(&cache, "nope").unwrap().is_none());
    }

    #[test]
    fn hit_count_increments_on_repeated_lookups() {
        let cache = tmp_cache();
        let entry = AiResponseCacheEntry {
            prompt_hash: "h".into(),
            provider: "openai".into(),
            model: "gpt-4".into(),
            prompt_preview: "q".into(),
            response: "a".into(),
            token_count_prompt: 100,
            token_count_completion: 50,
            created_at: 0,
            updated_at: 0,
            hit_count: 0,
            source_client: "host".into(),
        };
        upsert_response(&cache, &entry).unwrap();
        for expected in 1..=3i64 {
            let hit = lookup_response(&cache, "h").unwrap().unwrap();
            assert_eq!(hit.hit_count, expected, "hit should increment {expected}");
        }
    }

    #[test]
    fn stats_aggregates_correctly() {
        let cache = tmp_cache();
        for (hash, prov, tp, tc) in [
            ("h1", "claude_http", 100, 50),
            ("h2", "claude_http", 200, 100),
            ("h3", "openai", 300, 150),
        ] {
            upsert_response(
                &cache,
                &AiResponseCacheEntry {
                    prompt_hash: hash.into(),
                    provider: prov.into(),
                    model: "m".into(),
                    prompt_preview: "p".into(),
                    response: "r".into(),
                    token_count_prompt: tp,
                    token_count_completion: tc,
                    created_at: 0,
                    updated_at: 0,
                    hit_count: 0,
                    source_client: "host".into(),
                },
            )
            .unwrap();
        }
        // Hit h1 twice, h2 once, h3 zero
        let _ = lookup_response(&cache, "h1").unwrap();
        let _ = lookup_response(&cache, "h1").unwrap();
        let _ = lookup_response(&cache, "h2").unwrap();

        let s = stats(&cache).unwrap();
        assert_eq!(s.entry_count, 3);
        assert_eq!(s.total_hits, 3);
        // tokens_saved = (h1 2 hits × 100p + h2 1 hit × 200p) = 400
        assert_eq!(s.tokens_saved_prompt, 2 * 100 + 1 * 200);
        assert_eq!(s.tokens_saved_completion, 2 * 50 + 1 * 100);
        // providers: claude_http=2, openai=1
        assert_eq!(s.providers.len(), 2);
        assert_eq!(s.providers[0], ("claude_http".into(), 2));
        assert_eq!(s.providers[1], ("openai".into(), 1));
    }

    #[test]
    fn recent_entries_are_ordered_by_updated_at_desc() {
        let cache = tmp_cache();
        for (hash, _) in [("a", 0), ("b", 0), ("c", 0)] {
            upsert_response(
                &cache,
                &AiResponseCacheEntry {
                    prompt_hash: hash.into(),
                    provider: "claude_http".into(),
                    model: "m".into(),
                    prompt_preview: "p".into(),
                    response: "r".into(),
                    token_count_prompt: 0,
                    token_count_completion: 0,
                    created_at: 0,
                    updated_at: 0,
                    hit_count: 0,
                    source_client: "h".into(),
                },
            )
            .unwrap();
        }
        set_updated_at_for_test(&cache, "a", 10);
        set_updated_at_for_test(&cache, "b", 20);
        set_updated_at_for_test(&cache, "c", 30);
        // Re-hit `a` so it becomes most recent by updated_at without wall-clock sleeps.
        let _ = lookup_response(&cache, "a").unwrap();
        set_updated_at_for_test(&cache, "a", 40);

        let recent = recent_entries(&cache, 10).unwrap();
        assert_eq!(recent.len(), 3);
        assert_eq!(recent[0].prompt_hash, "a", "most recently touched is first");
    }

    #[test]
    fn upsert_replaces_existing_entry_same_hash() {
        let cache = tmp_cache();
        let mut e = AiResponseCacheEntry {
            prompt_hash: "same".into(),
            provider: "claude_http".into(),
            model: "m".into(),
            prompt_preview: "p".into(),
            response: "old".into(),
            token_count_prompt: 100,
            token_count_completion: 50,
            created_at: 0,
            updated_at: 0,
            hit_count: 0,
            source_client: "h1".into(),
        };
        upsert_response(&cache, &e).unwrap();
        e.response = "new".into();
        e.source_client = "h2".into();
        upsert_response(&cache, &e).unwrap();

        let hit = lookup_response(&cache, "same").unwrap().unwrap();
        assert_eq!(hit.response, "new", "second upsert replaces response");
        assert_eq!(hit.source_client, "h2");
    }

    #[test]
    fn estimate_tokens_is_roughly_one_per_four_chars() {
        assert_eq!(estimate_tokens(""), 0);
        assert_eq!(estimate_tokens("ab"), 1);
        assert_eq!(estimate_tokens("abcd"), 1);
        assert_eq!(estimate_tokens("abcde"), 2);
        assert_eq!(estimate_tokens("a".repeat(40).as_str()), 10);
    }

    #[test]
    fn prune_removes_old_entries_only() {
        let cache = tmp_cache();
        let long_ago = now_ts() - 100_000;
        upsert_response(
            &cache,
            &AiResponseCacheEntry {
                prompt_hash: "old".into(),
                provider: "p".into(),
                model: "m".into(),
                prompt_preview: "".into(),
                response: "r".into(),
                token_count_prompt: 0,
                token_count_completion: 0,
                created_at: long_ago,
                updated_at: long_ago,
                hit_count: 0,
                source_client: "".into(),
            },
        )
        .unwrap();
        upsert_response(
            &cache,
            &AiResponseCacheEntry {
                prompt_hash: "new".into(),
                provider: "p".into(),
                model: "m".into(),
                prompt_preview: "".into(),
                response: "r".into(),
                token_count_prompt: 0,
                token_count_completion: 0,
                created_at: 0,
                updated_at: 0,
                hit_count: 0,
                source_client: "".into(),
            },
        )
        .unwrap();

        let pruned = prune_older_than(&cache, 50_000).unwrap();
        assert_eq!(pruned, 1);
        assert!(lookup_response(&cache, "old").unwrap().is_none());
        assert!(lookup_response(&cache, "new").unwrap().is_some());
    }
}
