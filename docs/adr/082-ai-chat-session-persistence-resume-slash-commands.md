# ADR-082: AI chat session persistence + resume slash commands

## Status

Accepted — 2026-04-17.

## Context

Four AI chat surfaces exist in the terminal:

1. **Claude Code** (`claude --print --session-id <uuid>` / `--resume <uuid>`)
2. **Google AI CLI** (`agy --prompt ...` / `antigravity --prompt ...` preferred, `gemini --prompt ...` fallback — no native resume)
3. **Codex CLI** (`codex exec ...` — no native resume)
4. **Generic AI Chat** (HTTP to Claude/OpenAI/Gemini/Grok/Mistral/Perplexity/Local)

Before this change, **every conversation was ephemeral**: the transcript lived in a
`Vec<(bool, String)>` on the `App` struct and evaporated on restart. For Claude,
the CLI's own `--session-id` was also in-memory only, so even though Claude's
backend retained server-side context, we lost the key that would let us re-enter
that thread. This ran counter to the user's ask: *"can we save all AI sessions
somewhere, so we can attempt to resume them with RESUMEGEMINI, RESUMECLAUDE,
RESUMECODEX, RESUMEAI?"*

## Decision

Add an `ai_sessions` module plus four slash commands and a history-browser window.

### 1. Storage — kv_cache, zstd-compressed

Reuse `SqliteCache::put_kv` / `get_kv` (zstd-compressed; hot writes currently
use level 3). Two key
shapes:

- `ai:session:<provider>:<session_id>` → JSON of one `AiSessionRecord`
- `ai:sessions:index`                   → JSON `Vec<SessionIndexEntry>` (sorted
  DESC by `last_touched_at`, capped at 500)

```rust
pub struct AiSessionRecord {
    pub session_id: String,
    pub provider: String,          // "claude" | "gemini" | "codex" | "ai_chat"
    pub cli_session_id: String,    // Claude's --resume UUID; empty for others
    pub started_at: i64,
    pub last_touched_at: i64,
    pub turns: Vec<(bool, String)>,
    pub subject: String,           // first user message, trimmed to 120 chars
    pub model: String,
}
```

A separate index is used instead of scanning `ai:session:*` keys because
`kv_cache` has no prefix index — a LIKE scan would be O(n) on every open of the
history window.

### 2. Auto-save at reply receipt

Every time a reply is appended to any of the four histories (Claude / Gemini /
Codex live in `typhoon-native/src/app/ai.rs:235`, `:468`, `:664` after the ADR-086
split; AI Chat lives in `typhoon-native/src/app.rs:147576`), we call
`Self::persist_ai_turn(provider, session_id, cli_session_id, &history, model)`.
The first save for a non-Claude session generates a UUID via the existing
`new_uuid()` helper. Claude reuses its `--session-id` UUID as both the CLI
resume key and our kv key.

### 3. Resume slash commands

- `/RESUMECLAUDE` — `latest_for_provider(cache, "claude")`, restore the transcript
  *and* the `claude_code_session_id`, open the Claude Code window. The next
  `Send` already uses `--resume <id>` via the existing logic at
  `typhoon-native/src/app/ai.rs:426`.
- `/RESUMEGEMINI` — restore transcript into `gemini_cli_history`; no native
  resume, so the replayed transcript is injected via the existing
  `build_claude_prompt` call that already includes full history on every turn.
  The active command surface is now Antigravity/Gemini: `ASKANTIGRAVITY` is the
  primary palette command, `ASKGEMINI` remains a legacy alias, and the process
  launcher checks `agy`, then `antigravity`, before falling back to `gemini`.
- `/RESUMECODEX` — same strategy as Antigravity/Gemini.
- `/RESUMEAI` — restore into `ai_chat_history`; `AiChat` broker command already
  threads `history: Vec<(bool, String)>` into the API request.
- `/AISESSIONS` / `/AI_SESSIONS` — open the history browser window.

### 4. History browser

New window shows all saved sessions in a 6-column grid (provider, subject,
turns, model, last touched, actions). Actions per row:

- **View** — open the full transcript in-pane.
- **Resume** — restore the transcript into the target window and open it.
  Claude resumes with `cli_session_id` intact; others replay as context.

Index auto-refreshes every 10s while the window is open.

## Trade-offs

- **Index vs. scan.** 500-entry cap is generous (a heavy user at 10 sessions/day
  hits it in ~50 days; oldest drop silently). If users want unbounded history we
  can promote from kv_cache to a dedicated `ai_sessions` table with an index on
  `last_touched_at`.
- **Gemini/Codex have no true resume.** The transcript becomes the context on
  the next send, which works but costs tokens proportional to history length.
  The user explicitly accepted this: *"at the very least can we save a text
  transcript of the conversations to pre-load to the model?"*
- **Blocking DB writes on reply path.** `persist_turn` writes zstd-compressed
  JSON inside `update()`. For typical transcripts (< 50 turns, < 100 KB) this is
  sub-millisecond. If a reply was unusually large we'd notice a UI hitch; if
  that happens we move it to `tokio::task::spawn_blocking`.

## Test plan

`typhoon-engine/src/core/ai_sessions.rs` test module:

- `roundtrip_and_index` — save one turn, reload it, assert subject/model/index
  shape.
- `preserve_started_at_across_updates` — second save one second later must
  preserve the first `started_at` but update `last_touched_at`.
- `latest_for_provider_picks_most_recent` — two Claude sessions + one Gemini;
  `latest_for_provider("claude")` returns the newer Claude.
- `index_caps_at_max` — writing 520 sessions leaves the index at exactly 500.

All four pass. Full suite: 1190 tests, 0 failed.

## Paid-API note

All functionality here is local: kv_cache is SQLite + zstd, no external calls.
Claude/Gemini/Codex CLIs are user-installed subscription tools. Generic AI Chat
needs a user-supplied API key as before — unchanged by this ADR.
