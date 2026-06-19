# ADR-096: AI Return Path Auto-Ingest

**Status:** Accepted
**Date:** 2026-05-05
**Related:** ADR-080 (web research ingestion + packet viewer), ADR-082
(AI session persistence), ADR-083 (AI response cache),
`typhoon-native/src/app.rs`, `typhoon-native/src/app/ai.rs`

## Context

ADR-080 shipped the `===TYPHOON_INGEST===` Return Path format plus the
`INGEST_RESEARCH` paste window. That made AI-discovered web articles
cacheable and LAN-syncable, but built-in AI surfaces still required a
manual copy/paste step even though their final replies already pass
through the native app.

The earlier ADR-080 alternative called this "scrape subprocess stdout".
The current implementation no longer needs streaming stdout parsing:
Claude, Gemini, and Codex CLI calls all use final-response channels, and
hosted ASKAI replies already return through `BrokerMsg::JsonResult`.

## Decision

Automatically queue `BrokerCmd::IngestResearchArticles` whenever a built-in
AI reply contains `===TYPHOON_INGEST===`.

Implemented as one shared app helper:

- `maybe_queue_ingest_from_ai_response(agent, response)` checks for the
  Return Path marker.
- On match, it sends the full reply to the existing ingest broker path with
  `agent_override` set to `claude`, `gemini`, `codex`, or `ai_chat`.
- The existing ADR-080 broker handler still owns parsing, URL dedupe,
  SQLite writes to `research_web_articles`, promotion into `research_news`,
  LAN sync visibility, and NEWS-window refresh.

Hook points:

- Claude Code CLI response drain in `typhoon-native/src/app/ai.rs`
- Gemini CLI response drain in `typhoon-native/src/app/ai.rs`
- Codex CLI response drain in `typhoon-native/src/app/ai.rs`
- Hosted AI Chat `BrokerMsg::JsonResult("AiChat", ...)` receive arm in
  `typhoon-native/src/app.rs`

`INGEST_RESEARCH` remains available for external web UIs, copied
transcripts, and LAN remote ingest.

## Consequences

- Built-in AI investigations no longer need a paste step for Return Path
  articles.
- Manual and automatic ingest share the same parser and persistence path,
  so dedupe and LAN sync behavior remain identical.
- Cached AI responses that contain Return Path blocks are also safe to
  auto-ingest again because article dedupe is URL-based per symbol.

## Verification

- `cargo test --manifest-path typhoon-native/Cargo.toml parse_ask_args -- --nocapture`
- `cargo check --manifest-path typhoon-native/Cargo.toml`
