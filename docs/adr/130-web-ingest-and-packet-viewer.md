# ADR-130: Web Research Ingestion from AI Agents + RESEARCH_PACKET Viewer

**Status:** Accepted
**Date:** 2026-04-15
**Supersedes/extends:** ADR-108 through ADR-129
**Related:** `engine/src/core/research.rs`, `engine/src/core/lan_sync.rs`,
`native/src/app.rs`, `docs/RESEARCH_PACKET.md`

## Context

The Godel-parity arc (ADRs 108–129) built a dense set of per-symbol
quantitative surfaces that get glued into the research packet shipped
to AI agents (Claude, Gemini, ChatGPT) by ASKAI / ASKCLAUDE /
ASKGEMINI. The packet is a one-way artifact: TyphooN sends numbers
and domain context, the agent replies with a research note. Any web
search the agent performs — news, filings, transcripts, forum
discussion — lives only in the agent's reply and is lost to the
rest of the terminal the moment the user moves on.

That asymmetry is cheap to close. If we ask the agent, in the packet
itself, to emit a structured block of the web sources it consulted,
we can parse that block, cache it per-symbol in SQLite, and let LAN
sync distribute it to peer terminals. The cost to the agent is two
lines of instructions; the benefit to the user is a growing,
cross-session, cross-terminal research corpus that populates itself
every time anyone asks an AI to investigate a symbol.

Round 21 also flagged the research packet's lack of a **human-facing
viewer**. Up to now the packet has been a behind-the-scenes artifact
that ASKAI injects into an HTTP system prompt — users could read it
only by scrolling the raw request bodies in the logs or by saving
the clipboard output of ASKAI. A first-class viewer window with
section-level navigation makes the packet inspectable, auditable,
and shareable without having to round-trip through an AI.

Both features are small, independent, and complementary to the
Round-22 wiring that just landed, so they are bundled into a single
ADR-130 commit after the Round 22 commit.

## Decision

Ship two new features as a single bundle:

1. **Web research ingestion** — A new per-symbol table
   `research_web_articles`, a lenient parser for
   `===TYPHOON_INGEST===` fenced blocks in an AI agent's reply, a
   new `INGEST_RESEARCH` command / window where the user pastes the
   reply, a broker handler that merges parsed articles into the
   cache, LAN sync registration, and a **Return Path** footer added
   to the research packet that instructs the agent to emit the ingest
   block.

2. **RESEARCH_PACKET viewer window** — A new `RESEARCH_PACKET`
   command / window that runs the existing `investigate_symbols()`
   builder and displays the packet in a split-pane layout: a
   left-hand tree of markdown headers (H2/H3/H4) for navigation,
   and a right-hand scrollable monospace body. Clicking a header
   node filters the body to that section and its children;
   "Show All" restores the full text. Generated packets can be
   copied to clipboard or saved to a `.md` file via a native file
   dialog.

## Engine changes (`engine/src/core/research.rs`)

1. **Types**:
   - `WebArticle { title, url, source, published_at, summary,
     agent_used, ingested_at }` — one article record.
   - `IngestedArticlesSnapshot { symbol, articles: Vec<WebArticle> }`
     — per-symbol bag, serialized as JSON blob per the standard
     research-table pattern.
   - `INGESTED_ARTICLES_MAX = 50` — FIFO cap per symbol.

2. **Schema v23** — `create_research_tables_v23` (layered on v22)
   adds `research_web_articles` with the standard
   `(symbol TEXT PRIMARY KEY, snapshot_json TEXT, updated_at INTEGER)`
   shape and `idx_research_web_articles_updated` index.

3. **CRUD helpers**:
   - `upsert_ingested_articles(conn, symbol, snap)`
   - `get_ingested_articles(conn, symbol) -> Option<snap>`
   - `append_ingested_articles(conn, symbol, incoming) -> (added, total)`
     — merges new articles into the symbol's existing bag. Dedupes
     by URL case-insensitively. On conflict, the record with the
     larger `ingested_at` wins. After merging, the bag is sorted
     most-recent-first and truncated to `INGESTED_ARTICLES_MAX`.

4. **Parser**:
   - `parse_ingest_block(text) -> Vec<(String, Vec<WebArticle>)>`
     — lenient scanner for `===TYPHOON_INGEST===` … `===END_INGEST===`
     fenced blocks. Accepts `published` / `date` aliases for
     `published_at`, `agent` for `agent_used`, strips ```json
     fences, tolerates leading/trailing whitespace. Silently skips
     entries missing a symbol or URL. Supports multiple ingest
     blocks in a single reply. Returns grouped by uppercase symbol.

5. **Engine tests** (added to `core::research::tests`):
   - `ingested_articles_roundtrip` — upsert + get JSON roundtrip.
   - `ingested_articles_append_dedupe_and_cap` — verifies URL
     dedup, timestamp-wins replacement, and 50-entry cap.
   - `parse_ingest_block_extracts_articles` — happy path plus
     `published`/`date` alias handling, case-insensitive symbol.
   - `parse_ingest_block_with_json_fence` — tolerance for ```json
     / ``` wrapping.
   - `parse_ingest_block_skips_malformed_entries` — entries without
     symbol or URL are dropped without aborting the parse.
   - `parse_ingest_block_returns_empty_when_missing` — no ingest
     block → empty result.
   - **Test suite: 876 (Round 22) → 883 passing (+7 = 2 storage +
     5 parser).**

## LAN sync changes (`engine/src/core/lan_sync.rs`)

- Added `research_web_articles` to `SYNCABLE_TABLES` under the
  `// ── ADR-130 web article ingestion ──` divider.
- Added matching arm in `create_table_sql()` with the standard
  `(symbol, snapshot_json, updated_at)` DDL shape.
- Added matching arm in `table_timestamp_column()` mapping to
  `updated_at` for incremental sync.

Standalone clients, LAN clients, and LAN servers all pick up
ingested articles from each other on the next sync window —
whoever happens to be the "ingestor" for a given piece of news
populates the whole terminal farm.

## Native changes (`native/src/app.rs`)

### Ingest pipeline
- **1 new `BrokerCmd` variant**: `IngestResearchArticles { text,
  agent_override }`.
- **1 new `BrokerMsg` variant**: `IngestResearchResult {
  per_symbol_added, errors }`.
- **State fields** for the ingest window:
  - `show_ingest_research: bool`
  - `ingest_research_text: String`
  - `ingest_research_agent: String` (default `"claude"`; merged into
    records whose `agent_used` field is empty)
  - `ingest_research_status: String` (last result summary)
  - `ingest_research_busy: bool`
- **Broker handler**: spawns a Tokio task that calls
  `research::parse_ingest_block`, then for each `(symbol, articles)`
  pair calls `research::append_ingested_articles`, and sends an
  `IngestResearchResult` back.
- **Receive arm** in the BrokerMsg loop: sets
  `ingest_research_busy = false`, builds a human-readable summary
  of `per_symbol_added` (e.g. `AAPL: +3 (now 8) · MSFT: +1 (now 5)`),
  and pushes the summary to the log.
- **Palette entry**: `INGEST_RESEARCH | INGEST | RESEARCH_INGEST |
  INGESTRESEARCH` opens the window.
- **egui window**: multi-line paste area (monospace, 20 rows), a
  "Default agent tag" single-line field, an `Ingest` button (gated
  on non-empty text + not busy), a `Clear` button, a status line
  under the separator, and an explanatory header.

### Packet viewer
- **State fields**:
  - `show_packet_viewer: bool`
  - `packet_viewer_symbol: String` (comma-separated)
  - `packet_viewer_question: String`
  - `packet_viewer_text: String`
  - `packet_viewer_tree: Vec<PacketTreeNode>`
  - `packet_viewer_scroll_target: Option<usize>` (reserved; current
    impl uses section-filtering, not byte-level scrolling)
  - `packet_viewer_selected: Option<usize>`
- **New helper type**: `PacketTreeNode { depth: u8, title: String,
  byte_offset: usize }`.
- **New helper fn**: `Self::build_packet_tree(text) ->
  Vec<PacketTreeNode>` — scans the packet text line-by-line and
  captures H2/H3/H4 header rows with their byte offsets.
- **Palette entry**: `RESEARCH_PACKET | PACKET | PACKET_VIEW |
  VIEW_PACKET | RESEARCH_PACKET_VIEW` opens the window. Pre-fills
  the symbol input from the active chart tab if empty.
- **egui window** (980×680 default):
  - Top row: `Symbols` input, `Use Chart` button, `Question`
    (optional) input, `Generate` button, `Copy` button, `Save…`
    button (native file dialog → .md file).
  - Body: left-hand `Panel::left` tree nav with indented
    selectable labels for H2/H3/H4 depth levels, "Show All"
    button that clears the selection, and a header-count/byte-
    count summary; right-hand `CentralPanel::default` scrollable
    monospace `TextEdit::multiline` in code-editor mode showing
    either the full packet (no selection) or the slice from the
    selected section's offset to the start of the next section at
    the same-or-shallower depth.
  - Text is read-only: local edits to the displayed buffer are
    not written back to `packet_viewer_text`.

### Packet builder additions (`investigate_symbols()`)
- **Per-symbol `INGESTED` block**: after the Round 22 DAYRANGE
  packet block, read `research::get_ingested_articles` for the
  symbol and emit a `### Prior Ingested Web Research — INGESTED
  (N articles)` section listing the top 15 articles (title, source,
  published date, agent, summary truncated to 260 chars, URL) with
  a "(N more in cache, not shown)" footer if there are more than
  15. Silent if the cache is empty.
- **Return Path footer**: after the closing Question, emit a
  `## Return Path — Web Research Ingest` section with a fenced
  code block showing the exact JSON shape the agent should emit
  and a short rules paragraph. The agent is instructed to include
  one object per distinct article, to flag every symbol the article
  references, and to synthesize (not copy-paste) the summary.

## Research packet changes (`docs/RESEARCH_PACKET.md`)

- New section describing the `INGESTED` per-symbol sub-block
  (section 2.102, renumbered the sector-peer comparison to 2.103).
- New section describing the Return Path footer.
- New size-caps row and new data source row for
  `research::get_ingested_articles`.
- Added RESEARCH_PACKET viewer to the command-surface table.
- Updated envelope: ingest footer adds ~600 bytes/packet; ingested
  articles block adds ~1.5 KB/symbol when populated.
- Added ADR-130 to the Related list.

## Alternatives considered

1. **Scrape the LLM subprocess stdout for fenced ingest blocks in
   ASKCLAUDE / ASKGEMINI** — Rejected for this ADR, but flagged as
   an obvious Round-2 follow-up. Would eliminate the paste step
   entirely for subprocess paths. Not included here because it
   requires extending the subprocess reader loop and teasing apart
   agent-reply-vs-streaming-output for each CLI, and because the
   paste path needs to exist anyway for HTTP/web-UI agents that
   can't be wrapped in a subprocess.
2. **Ingesting plain URL lists (no JSON)** — Rejected. Plain URLs
   lose the title / source / summary / published_at metadata that
   makes the cache searchable and useful. Accepting raw URL lines
   as a fallback was considered and dropped to keep the parser
   simple — if an agent can't emit JSON it can't emit the ingest
   block at all.
3. **Per-symbol sub-tables vs single JSON-blob table** — Matching
   the existing convention: one row per symbol, articles serialized
   as a JSON array inside `snapshot_json`. Allows LAN sync to reuse
   the standard `(symbol TEXT PRIMARY KEY, snapshot_json TEXT,
   updated_at INTEGER)` path with zero custom code. A normalized
   `(article_id, url, symbol_fk)` schema would be cleaner but would
   need bespoke LAN sync handlers — not worth it for a bag that
   tops out at 50 entries per symbol.
4. **Storing article *content*, not just metadata** — Rejected.
   Full article bodies bloat the packet and the cache, raise
   licensing concerns, and the agent's `summary` field is exactly
   the synthesis we want to retain. The URL is preserved so the
   user can click through to the source.
5. **Dedup key on `(url, symbol)` vs `url` alone** — Picked URL
   alone, within a symbol's bag. Two symbols that both reference
   the same article will each have their own entry in their own
   snapshot; the duplication is cheap and lets per-symbol views
   render cleanly without a join.
6. **50-entry cap vs unlimited / age-based pruning** — Picked a
   hard cap because LAN sync's cost scales with row size, not row
   count, and a bag of 50 max is still <30 KB typical JSON. Age-
   based pruning would also work but adds a background job; the
   FIFO-on-insert is simple and deterministic.
7. **Packet viewer scrolling via byte-offset → line-number** —
   Implemented and then discarded. egui's `TextEdit::multiline`
   does not expose a scroll-to-line API that works reliably with
   a shared mutable string buffer, and `ui.scroll_to_cursor` fires
   on the wrong widget when called outside a cursor-setting
   callback. The section-filter approach is strictly simpler: click
   an H2, see the whole H2 block and its children; click an H3,
   see just that H3. "Show All" returns to the full view. This is
   actually the better UX — large packets are less cognitively
   overwhelming when you see one section at a time.
8. **Write the packet to a temp file and open it in `$EDITOR`** —
   Rejected. Works for advanced users but breaks the self-contained
   terminal experience. A native in-app viewer is table-stakes for
   a trading terminal.
9. **Render markdown with egui's richtext formatter** — Rejected
   for v1. egui does not have a production-grade markdown renderer;
   maintaining our own would dwarf the rest of the feature. The
   monospace code-editor view is honest and works.

## Consequences

- **Ingest corpus grows passively**: Every research session an AI
  agent runs (across any terminal on the LAN) adds to the shared
  article cache. Over weeks a basket of 20 actively-tracked symbols
  will accumulate 200–1000 articles with zero user effort beyond
  pasting the agent's reply once per session.
- **Packets are audit-friendly**: `RESEARCH_PACKET SYM` gives a
  clear view of exactly what was sent to the AI — useful for
  debugging prompt regressions, teaching, and compliance review.
- **LAN sync surface area**: +1 table. Negligible bandwidth impact
  (≤30 KB per symbol bag in steady state).
- **Schema migration**: Schema v23. Created automatically on
  upsert / get, same lazy-migration pattern used by every prior
  schema version.
- **No new external dependencies**: The parser uses `serde_json`
  which is already in `Cargo.toml`. The viewer uses only `egui`,
  `chrono`, and `rfd` (already used for chart export).
- **Agent compliance is voluntary**: Agents that ignore the Return
  Path footer simply don't populate the cache. No error, no
  regression — the cache just stays at its existing size.
- **Cross-symbol linking**: An article referenced by multiple
  symbols (e.g. a macro story touching AAPL + MSFT + NVDA) is
  stored in each symbol's bag independently. The deduplication is
  per-symbol, not global. This is intentional — it keeps the bag
  view coherent even when LAN peers ingest asymmetrically.

## Implementation notes

- **`parse_ingest_block` is deliberately tolerant**: Missing
  `source`, `summary`, or `agent` are fine. Missing `symbol` or
  `url` drops the record. `published_at` accepts free-form strings
  — downstream consumers render whatever is stored. The parser
  does NOT validate URLs or dates because AI agents produce a wide
  spectrum of malformed output; it's better to cache a slightly
  messy record than to drop a useful one over a trailing period.
- **`append_ingested_articles` sort-by-timestamp**: After the merge
  pass the articles are sorted with `b.ingested_at.cmp(&a.ingested_at)`
  (descending) and truncated to `INGESTED_ARTICLES_MAX`. This means
  the FIFO drop targets the *oldest* ingests, not the oldest
  articles by `published_at`, which is the right behavior for a
  research cache — we want to keep the most recently *processed*
  view of the corpus, regardless of when the source was published.
- **`ingested_at = 0` sentinel**: If an incoming article has
  `ingested_at == 0` the merge sets it to `now_ts()` before storing.
  This handles the parser's default (which leaves it 0 because the
  agent obviously can't know the ingestion time) without requiring
  every call site to remember to set it.
- **`agent_override` semantics**: The native ingest handler passes
  the `ingest_research_agent` field value as `agent_override`. The
  handler loops over parsed articles and fills in `agent_used` from
  the override *only if the article's own field is empty*. So an
  agent that explicitly reports `"agent": "claude"` in its ingest
  block is trusted; an agent that omits the field gets tagged
  with whatever the user has set in the window.
- **Packet viewer section filtering**: When an H2 is selected, the
  slice extends until the next H2 (or end of text). When an H3 is
  selected, the slice extends until the next H3 OR H2 — whichever
  comes first, which is the natural "this section and its H4
  children only" boundary. The slice uses byte-level indexing
  into the source `String`; header offsets are captured at
  tree-build time on line boundaries so slices always start at
  valid UTF-8 code points.
- **Packet viewer read-only buffer trick**: The body is rendered
  in a standard `TextEdit::multiline` that we clone the source
  string into on every frame. Local edits to the cloned buffer
  are silently dropped because `self.packet_viewer_text` is never
  re-assigned from the widget state. This gives us the code-editor
  font / selection / cursor / copy-paste behavior "for free"
  without a mutable source of truth. The penalty is one string
  clone per frame, which is ≤100 KB for a 20-symbol basket and
  unnoticeable.
- **File dialog default name**: Save defaults to
  `research_packet_<symbols>_<yyyymmdd_hhmmss>.md` with commas
  replaced by underscores. Avoids overwrites and makes multi-run
  batches easy to correlate to the source symbols.

## Test coverage

- 7 new engine tests (5 parser + 2 storage). See "Engine tests"
  above.
- Build verification: `cargo build -p typhoon-native` clean, zero
  warnings after deprecation fixes.
- Engine test suite: 876 → 883 passing.
- No native tests for the two new windows — they are UI glue, same
  as every other Round N window. The underlying parser/store paths
  are fully covered.

## Future work

1. **Auto-parse ingest blocks in ASKCLAUDE/ASKGEMINI subprocess
   output** — The CLI subprocess readers already capture the agent
   reply into a buffer; a single call to `parse_ingest_block` on
   that buffer could feed `append_ingested_articles` automatically.
   Same thing in the HTTP ASKAI path once the final reply text is
   in scope.
2. **Article search surface** — A `RESEARCH_SEARCH <query>`
   command that runs full-text search over `articles_json` across
   all symbols, similar to how the existing news-search window
   works but over the ingested corpus.
3. **Packet viewer: rendered markdown** — If egui gains a
   production markdown widget, swap the monospace TextEdit for it.
   Until then, the current view is fine.
4. **Packet viewer: diff two packets** — Useful for quality
   regression testing of the Godel-parity arc. Would need a second
   `packet_viewer_text_b` field and a unified-diff renderer. Not
   in v1.
5. **Ingest ACL per-agent** — A setting to whitelist/blacklist
   which agents are allowed to write to the cache (e.g. only
   Claude, not random ChatGPT pastes from a coworker). Not needed
   on day one.
6. **LAN sync priority for articles** — Currently all synced
   tables are polled on the same cadence. Articles are slightly
   more write-heavy than other research tables; a faster cadence
   for `research_web_articles` would spread ingests across the
   LAN sooner. Not a v1 concern.
