# ADR-162: Cross-Client AI Response Cache

**Status:** Accepted
**Date:** 2026-04-17
**Supersedes/extends:** ADR-157 (AI session persistence)
**Related:** `engine/src/core/ai_response_cache.rs`,
`engine/src/core/lan_sync.rs`, `native/src/app.rs`

## Context

The user runs the Typhoon Terminal across several LAN-attached machines
and routinely pastes the same or near-same research packet into the AI
chat pane on each one. Every paste is billed tokens — the laptop pays,
the workstation pays, the secondary monitor machine pays. For a canonical
10-symbol basket packet (~1.4 MB, ~350 k tokens) this compounds quickly,
and the answers are almost always identical across clients because the
prompt is deterministic in the packet content.

Two things already exist in this area:

1. **AI session persistence** (ADR-157) — transcripts are stored in
   `kv_cache` as zstd blobs. Accuracy update 2026-05-05: although
   `kv_cache` is not part of `SYNCABLE_TABLES`, LAN sync has a separate
   `RequestKvData` path that replicates KV rows while filtering
   credentials, LAN-local config, quote churn, and other machine-local
   keys. AI session keys (`ai:session:*`, `ai:sessions:index`) therefore
   replicate today through the KV path, not through the table whitelist.
2. **Web article cache** (`research_web_articles`) — already LAN-synced,
   demonstrating the pattern of dedicating a regular table to
   cross-client deduplication rather than trying to re-key the KV store.

Neither of these solves the "one machine pays, every machine benefits"
problem for AI responses specifically. The user's standing directive is
to minimise token spend across all clients.

A simple SHA256 over the normalised prompt tuple (provider, model,
system, history, message) is sufficient for deduplication:

- The packet content changes every day (new prices, new timestamps),
  which naturally invalidates old entries by changing the hash.
- Whitespace normalisation around the boundaries is safe (trim only),
  since internal newlines carry semantic meaning in packets.
- No provider-specific tokenizer is shipped; a `chars/4` rule-of-thumb
  is good enough for the "tokens saved" display.

## Decision

Ship a new regular table `ai_response_cache` with the standard LAN-sync
shape, and intercept every AI call in the native layer to check the
cache before spending tokens.

### Schema

```
ai_response_cache (
    prompt_hash TEXT PRIMARY KEY,          -- hex sha256 of the tuple
    provider TEXT NOT NULL,                -- claude_http | openai | gemini | grok | mistral | perplexity | local | *_cli
    model TEXT NOT NULL,                   -- provider-specific model id at call time
    prompt_preview TEXT NOT NULL,          -- last-user-message trimmed to ~400 chars
    response TEXT NOT NULL,
    token_count_prompt INTEGER NOT NULL,   -- estimate_tokens() over system+message
    token_count_completion INTEGER NOT NULL,
    created_at INTEGER NOT NULL,           -- unix seconds, set once at first insert
    updated_at INTEGER NOT NULL,           -- unix seconds, bumped on every hit
    hit_count INTEGER NOT NULL,            -- incremented on every lookup_response hit
    source_client TEXT NOT NULL            -- originating hostname (from $HOSTNAME)
);
CREATE INDEX idx_ai_response_cache_updated ON ai_response_cache (updated_at DESC);
CREATE INDEX idx_ai_response_cache_provider_model ON ai_response_cache (provider, model);
```

### Prompt hash normalisation

```rust
hash_ai_prompt(provider, model, system, history, message) -> hex sha256
    input bytes: "v1\0" | provider.trim()\0 | model.trim()\0 | system.trim()\0 |
                 for each (is_user, text) in history: ("u\0" | "a\0") | text.trim()\0 |
                 "msg\0" | message.trim()
```

Rationale:

- `v1\0` prefix lets us re-version later without colliding with v1
  entries sitting on peer disks.
- `\0` separators prevent "concatenation confusion" attacks (A|B vs AB).
- `u\0 / a\0` prefixes for history turns preserve role — a
  user-message vs an assistant-message of the same text must hash
  differently.
- `trim()` on each field swallows leading/trailing whitespace noise
  (editor trailing newlines, cursor-paste whitespace) without altering
  internal structure.
- SHA256 because we do not need cryptographic strength — we need
  deterministic bit-exact collision avoidance between LAN peers. SHA256
  is ubiquitous, in-tree via `sha2`, and gives 64-char hex primary keys
  that are comfortable to read in the stats window.

### LAN sync

`ai_response_cache` added to `SYNCABLE_TABLES` with
`table_timestamp_column = updated_at` and the standard CREATE TABLE
stanza in `create_table_sql`. Hits on one peer refresh `updated_at`,
which causes the next delta sync window to ship the updated hit_count
back out — so peers converge on a shared view of "which entries are
being actively used."

### Native intercept (cache-aside)

In the `BrokerCmd::AiChat` tokio task:

```
1. compute prompt_hash = hash_ai_prompt(...)
2. snapshot = shared_cache_broker.read() — one clone of the Arc
3. if let Some(cache) = &snapshot {
       if let Ok(Some(hit)) = lookup_response(cache, &prompt_hash) {
           send BrokerMsg::JsonResult("AiChat", hit.response);
           return; // skip the HTTP call
       }
   }
4. proceed with HTTP call
5. on successful text response (not "(no response)"):
       upsert_response(cache, { prompt_hash, response, token counts,
                                source_client = $HOSTNAME, ... })
```

The intercept sits at the same tokio-task boundary the existing Claude
and OpenAI-compatible branches already share, so both paths get caching
for free. The cache is best-effort: lookup failures log-and-proceed, and
upsert failures never block the response delivery.

### AICACHE window

New palette command `AICACHE | AI_CACHE | AI_RESPONSE_CACHE |
RESPONSE_CACHE` opens an `egui::Window` showing:

- Aggregate stats (entry count, total hits, tokens saved — prompt and
  completion separately).
- Provider breakdown (sorted desc by entry count).
- Recent entries grid (updated_at DESC, limit 50): provider, model,
  hash, hit count, source client, prompt preview.
- Refresh button and Prune-older-than-30-days button.

Auto-refreshes every 10 seconds while the window is open.

### No-API-dependency invariant preserved

The cache is purely derived state over the free-API surface — no new
network dependencies, no paid-API calls, no vendor SDKs. The
`estimate_tokens` function is `chars / 4` rule-of-thumb with no provider
tokenizer dependency.

## Consequences

### Positive

- **Token spend minimised.** The workstation runs a packet → the laptop
  opens the same packet five minutes later → cache hit → zero tokens.
  Cost reduction scales with client count: N clients on the same LAN
  running the same basket → ~1/N token spend.
- **LAN-sync convergence.** The hit_count bump is itself synced, so
  peers learn which queries are popular across the whole LAN and can
  make informed prune decisions.
- **Debuggable.** The stats window exposes the cache in cleartext
  (preview + response) so the user can inspect what the cache decided
  was a hit. A silent cache with opaque hash-only stats would be
  operationally frustrating.
- **Additive schema.** New table, new code path, no existing surface
  modified. Rolls back cleanly by not invoking the intercept.

### Negative / Risks

- **Privacy on shared LAN.** Responses are stored in cleartext and
  replicated to every LAN peer via `lan_sync`. This is by design: the
  user explicitly wants cross-client dedup across their own machines,
  and the LAN environment is trust-boundary-equivalent to the user's
  own filesystem. A multi-user LAN that mixed the same `typhoon_cache/`
  bind-mount across untrusted hosts would leak queries — that is out of
  scope and the LAN sync protocol itself makes the same assumption.
- **Staleness vs freshness.** Packets are daily; a stale cache hit
  during the same-day window returns yesterday's numbers if the user
  pasted yesterday's packet today. Mitigated because the packet content
  *is* part of the hash — only bit-identical packets collide, and the
  daily timestamp inside the packet changes every day. The 30-day prune
  button addresses long-tail entries for users who chat the same
  questions repeatedly over weeks.
- **`hit_count` LAN-sync contention.** Two peers could increment
  hit_count on the same entry near-simultaneously and the last-writer-
  wins behaviour of the delta protocol loses one increment. Acceptable
  — the counter is for display, not billing.
- **Hash tuple does not include packet-age.** If the AI system prompt
  changes structure silently (e.g. we add a new sub-block), all
  previously cached entries miss. This is the *correct* behaviour and
  matches the intent — but users will see a one-time cold-cache spike
  on rollout days.

### Neutral

- `estimate_tokens` is coarse. Real-token-count accuracy would require
  per-provider tokenizers (tiktoken for OpenAI, anthropic-tokenizer for
  Claude, etc.). Not shipped because the cost/value is inverted: the
  stats window is advisory, and the cache lookup itself does not depend
  on the estimate.
- CLI providers (claude_cli, gemini_cli, codex_cli) are also cached.
  These cost no API tokens but may cost compute — caching is still net-
  positive because subsequent replays are instant.

### Paid-API gap

None introduced. Standing godel-parity directive preserved.

## Verification

- `cargo test -p typhoon-engine --lib ai_response_cache`: 11 tests pass.
  - hash_is_deterministic
  - hash_changes_on_any_input_change
  - hash_ignores_leading_trailing_whitespace
  - roundtrip_insert_and_lookup
  - lookup_miss_returns_none
  - hit_count_increments_on_repeated_lookups
  - stats_aggregates_correctly
  - recent_entries_are_ordered_by_updated_at_desc
  - upsert_replaces_existing_entry_same_hash
  - estimate_tokens_is_roughly_one_per_four_chars
  - prune_removes_old_entries_only
- `cargo build -p typhoon-native`: clean build.
- Full engine test suite: 1244 tests pass (1233 prior + 11 new).

## Follow-ups

- **Per-symbol cache invalidation hook.** When the user runs `REFRESH`
  on a symbol, any cache entries whose `prompt_preview` or `response`
  mentions that symbol could be proactively pruned. Today they simply
  age out and re-resolve on the next run.
- **Provider-specific tokenizers.** If token-accounting precision
  becomes important (e.g. for billing reconciliation), swap in real
  tokenizers per provider. Currently the rough estimate is sufficient
  for the stats-window advisory role.

Closed 2026-05-05: the old "AI session kv_cache -> table migration"
follow-up was based on the mistaken assumption that KV rows did not
LAN-sync. A regular `ai_sessions` table may still be useful later for
querying, pagination, or a stricter privacy toggle, but it is not needed
for cross-client availability.
