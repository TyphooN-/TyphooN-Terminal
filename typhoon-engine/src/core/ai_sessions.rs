//! AI chat session persistence.
//!
//! Stores transcripts of Claude Code / Gemini CLI / Codex CLI / generic AI Chat
//! sessions in the existing zstd-compressed KV cache so the user can resume a
//! past conversation across restarts.
//!
//! Layout:
//! - `ai:session:<provider>:<session_id>` → JSON of [`AiSessionRecord`]
//! - `ai:sessions:index`                  → JSON `Vec<SessionIndexEntry>`,
//!   sorted by `last_touched_at` DESC, capped at `MAX_INDEX` entries (oldest dropped).
//!
//! The index is the only O(1)-accessible list for the history window — we don't
//! scan `ai:session:*` keys, since `kv_cache` has no prefix index.
//!
//! `persist_turn` is the single mutation entrypoint: it reads the existing record
//! (if any), preserves `started_at`, updates the transcript + timestamp + subject,
//! and rewrites both the record and the index in one call.
//!
//! Native-side note: Claude Code already has its own CLI session UUID (passed to
//! `claude --resume`). We store it in `cli_session_id` so RESUMECLAUDE can restore
//! the `--resume` chain after a restart. Gemini/Codex have no native resume so
//! `cli_session_id` stays empty for them — the saved transcript is replayed into
//! the chat pane and used as prompt context on the next turn.
//!
//! See for the full resume UX.

/// Default Gemini CLI model id. Shared by the native app's model selector and the
/// broker AI-chat handler (ADR-125 Target 3 — keeps the broker processor engine/std-only).
pub const DEFAULT_GEMINI_CLI_MODEL: &str = "gemini-3.1-pro-preview";

use crate::core::cache::SqliteCache as Cache;
use serde::{Deserialize, Serialize};

/// One persisted AI conversation.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AiSessionRecord {
    pub session_id: String,
    pub provider: String,       // "claude" | "gemini" | "codex" | "ai_chat"
    pub cli_session_id: String, // native Claude --resume id; empty for others
    pub started_at: i64,        // unix seconds
    pub last_touched_at: i64,
    pub turns: Vec<(bool, String)>, // (is_user, message)
    pub subject: String,            // first user message trimmed to ~120 chars
    pub model: String,              // model name at time of save
}

/// One row in the session-history index. Small so the whole index fits in
/// one kv_cache value without paging.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionIndexEntry {
    pub session_id: String,
    pub provider: String,
    pub started_at: i64,
    pub last_touched_at: i64,
    pub subject: String,
    pub turn_count: usize,
    pub model: String,
}

const INDEX_KEY: &str = "ai:sessions:index";
const MAX_INDEX: usize = 500;

fn now_ts() -> i64 {
    chrono::Utc::now().timestamp()
}

fn session_key(provider: &str, session_id: &str) -> String {
    format!("ai:session:{}:{}", provider, session_id)
}

fn pick_subject(turns: &[(bool, String)]) -> String {
    for (is_user, msg) in turns.iter() {
        if *is_user {
            let t = msg.trim();
            return t.chars().take(120).collect();
        }
    }
    String::new()
}

/// Persist a turn of an AI conversation. Handles both fresh and continuing
/// sessions — if the record already exists, `started_at` and any non-empty
/// prior `subject` are preserved.
pub fn persist_turn(
    cache: &Cache,
    session_id: &str,
    provider: &str,
    cli_session_id: Option<&str>,
    turns: &[(bool, String)],
    model: &str,
) -> Result<(), String> {
    if session_id.trim().is_empty() {
        return Err("empty session_id".into());
    }
    if provider.trim().is_empty() {
        return Err("empty provider".into());
    }

    let key = session_key(provider, session_id);
    let wall_now = now_ts();

    let existing = match cache.get_kv(&key) {
        Ok(Some(s)) => serde_json::from_str::<AiSessionRecord>(&s).ok(),
        _ => None,
    };

    // Read the resume index before choosing timestamps. Unix-second wall-clock
    // stamps can collide across freshly-created sessions, so use the index's
    // current max as a cheap monotonic floor for provider-local ordering.
    let mut idx = read_index(cache).unwrap_or_default();
    let provider_max_touched = idx
        .iter()
        .filter(|e| e.provider == provider)
        .map(|e| e.last_touched_at)
        .max();

    let started_at = existing.as_ref().map(|r| r.started_at).unwrap_or(wall_now);
    // Unix-second timestamps can collide during fast consecutive writes. Keep
    // session/provider updates monotonic so resume ordering and tests never
    // need sleeps.
    let base_now = existing
        .as_ref()
        .map(|r| wall_now.max(r.last_touched_at + 1))
        .unwrap_or(wall_now);
    let now = provider_max_touched
        .map(|max_seen| base_now.max(max_seen + 1))
        .unwrap_or(base_now);
    let subject = existing
        .as_ref()
        .map(|r| r.subject.clone())
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| pick_subject(turns));

    let rec = AiSessionRecord {
        session_id: session_id.to_string(),
        provider: provider.to_string(),
        cli_session_id: cli_session_id.unwrap_or("").to_string(),
        started_at,
        last_touched_at: now,
        turns: turns.to_vec(),
        subject: subject.clone(),
        model: model.to_string(),
    };

    let json = serde_json::to_string(&rec).map_err(|e| format!("serialize session: {e}"))?;
    cache.put_kv(&key, &json)?;

    // Update the index — replace any existing entry for the same (provider, session_id),
    // then push the fresh entry and resort by most-recent-first.
    idx.retain(|e| !(e.session_id == rec.session_id && e.provider == rec.provider));
    idx.push(SessionIndexEntry {
        session_id: rec.session_id.clone(),
        provider: rec.provider.clone(),
        started_at: rec.started_at,
        last_touched_at: rec.last_touched_at,
        subject,
        turn_count: rec.turns.len(),
        model: rec.model.clone(),
    });
    idx.sort_by(|a, b| b.last_touched_at.cmp(&a.last_touched_at));
    if idx.len() > MAX_INDEX {
        idx.truncate(MAX_INDEX);
    }
    let idx_json = serde_json::to_string(&idx).map_err(|e| format!("serialize index: {e}"))?;
    cache.put_kv(INDEX_KEY, &idx_json)?;

    Ok(())
}

/// Load a specific session by (provider, session_id). Returns None if not found.
pub fn load_session(
    cache: &Cache,
    provider: &str,
    session_id: &str,
) -> Result<Option<AiSessionRecord>, String> {
    let key = session_key(provider, session_id);
    match cache.get_kv(&key)? {
        Some(s) => Ok(serde_json::from_str(&s).ok()),
        None => Ok(None),
    }
}

/// Read the session-history index. Empty on first run.
pub fn read_index(cache: &Cache) -> Result<Vec<SessionIndexEntry>, String> {
    match cache.get_kv(INDEX_KEY)? {
        Some(s) => Ok(serde_json::from_str(&s).unwrap_or_default()),
        None => Ok(Vec::new()),
    }
}

/// Find the most recently touched session for the given provider.
///
/// Used by the RESUME* slash commands to pick up where the user left off
/// without having to ask them which session.
pub fn latest_for_provider(
    cache: &Cache,
    provider: &str,
) -> Result<Option<AiSessionRecord>, String> {
    let idx = read_index(cache)?;
    for entry in idx.iter() {
        if entry.provider == provider {
            return load_session(cache, provider, &entry.session_id);
        }
    }
    Ok(None)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tmp_cache() -> Cache {
        let mut p = std::env::temp_dir();
        p.push(format!(
            "typhoon_ai_sessions_{}.db",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = std::fs::remove_file(&p);
        Cache::open(&p).expect("open cache")
    }

    fn force_index_time(cache: &Cache, provider: &str, session_id: &str, last_touched_at: i64) {
        let mut idx = read_index(cache).unwrap();
        for entry in &mut idx {
            if entry.provider == provider && entry.session_id == session_id {
                entry.last_touched_at = last_touched_at;
            }
        }
        idx.sort_by(|a, b| b.last_touched_at.cmp(&a.last_touched_at));
        cache
            .put_kv(INDEX_KEY, &serde_json::to_string(&idx).unwrap())
            .unwrap();
    }

    #[test]
    fn roundtrip_and_index() {
        let cache = tmp_cache();
        let turns = vec![
            (true, "what's AAPL doing?".into()),
            (false, "trading sideways".into()),
        ];
        persist_turn(
            &cache,
            "sess-1",
            "claude",
            Some("cli-uuid-1"),
            &turns,
            "opus",
        )
        .unwrap();

        let loaded = load_session(&cache, "claude", "sess-1").unwrap().unwrap();
        assert_eq!(loaded.turns.len(), 2);
        assert_eq!(loaded.cli_session_id, "cli-uuid-1");
        assert_eq!(loaded.subject, "what's AAPL doing?");
        assert_eq!(loaded.model, "opus");

        let idx = read_index(&cache).unwrap();
        assert_eq!(idx.len(), 1);
        assert_eq!(idx[0].turn_count, 2);
    }

    #[test]
    fn preserve_started_at_across_updates() {
        let cache = tmp_cache();
        let t1 = vec![(true, "first".into())];
        persist_turn(&cache, "sess-2", "gemini", None, &t1, "gemini-2.5-pro").unwrap();
        let first_started = load_session(&cache, "gemini", "sess-2")
            .unwrap()
            .unwrap()
            .started_at;

        let t2 = vec![(true, "first".into()), (false, "reply".into())];
        persist_turn(&cache, "sess-2", "gemini", None, &t2, "gemini-2.5-pro").unwrap();
        let second = load_session(&cache, "gemini", "sess-2").unwrap().unwrap();

        assert_eq!(second.started_at, first_started);
        assert!(second.last_touched_at > first_started);
        assert_eq!(second.turns.len(), 2);
    }

    #[test]
    fn latest_for_provider_picks_most_recent() {
        let cache = tmp_cache();
        persist_turn(
            &cache,
            "s-a",
            "claude",
            None,
            &[(true, "old".into())],
            "opus",
        )
        .unwrap();
        force_index_time(&cache, "claude", "s-a", 1);
        persist_turn(
            &cache,
            "s-b",
            "claude",
            None,
            &[(true, "new".into())],
            "sonnet",
        )
        .unwrap();
        force_index_time(&cache, "claude", "s-b", 2);
        persist_turn(
            &cache,
            "other",
            "gemini",
            None,
            &[(true, "other-prov".into())],
            "gemini-2.5-pro",
        )
        .unwrap();

        let latest = latest_for_provider(&cache, "claude").unwrap().unwrap();
        assert_eq!(latest.session_id, "s-b");
        assert_eq!(latest.turns[0].1, "new");
    }

    #[test]
    fn index_caps_at_max() {
        let cache = tmp_cache();
        for i in 0..(MAX_INDEX + 20) {
            let sid = format!("s-{}", i);
            persist_turn(
                &cache,
                &sid,
                "ai_chat",
                None,
                &[(true, format!("msg {i}"))],
                "claude",
            )
            .unwrap();
        }
        let idx = read_index(&cache).unwrap();
        assert_eq!(idx.len(), MAX_INDEX);
    }
}
