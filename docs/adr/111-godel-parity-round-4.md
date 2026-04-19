# ADR-111 — Godel Parity Round 4: Splits, ETF Holdings, Analyst Ratings, ESG, Index Membership + AI Chat Overhaul

**Status:** Implemented
**Date:** 2026-04-14

## Parity classification

| Feature | Godel documented? | TA-Lib primitive? | Research packet | egui popup | Chart overlay |
| --- | --- | --- | --- | --- | --- |
| SPLT (stock split history) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| ETF (holdings breakdown) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| ANR (analyst recommendations + PT) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| ESG (ESG score) | Yes | No | Yes | Yes | No (deferred — ADR-188) |
| MEMB (index membership) | Yes | No | Yes | Yes | No (deferred — ADR-188) |

**Round classification:** pure Godel-Terminal-documented research windows (splits, ETF holdings, analyst consensus, ESG, index membership); AI chat overhaul is UX infrastructure, not a parity feature.

## Context

ADR-108/109/110 closed most of the professional-desk research surface a Godel
user would reach for on a new ticker. What remained after Round 3 were five
surfaces with no TyphooN equivalent, plus a cluster of painful regressions in
the way the AI chat windows treated context and session state:

**New Godel-parity surfaces still missing:**

1. **SPLT — Stock split history.** The terminal had no way to see a symbol's
   historical split calendar. Matters for any post-split technical analysis
   (adjusting historical bars) and for derivative pricing on names with
   reverse splits.
2. **ETF — Holdings breakdown.** The terminal could show price, fundamentals,
   and news for an ETF but not its *constituents* — so "what's actually inside
   SPY vs VTI" required an external tool.
3. **ANR — Analyst recommendations + consensus price target.** UPDG (ADR-109)
   showed individual upgrade/downgrade events; ANR is the aggregate view —
   how many analysts are Buy/Hold/Sell this month, and what's the mean /
   median / range of their 12-month price targets.
4. **ESG — Environmental / Social / Governance score.** A single-number
   sustainability read — increasingly load-bearing for institutional
   allocation decisions, invisible to TyphooN users until now.
5. **MEMB — Index membership.** For any equity, "is it in the S&P 500 / NDX /
   DJIA, and when was it added?" — and for any index, "what companies are
   currently members?". The missing surface closed the loop between
   single-ticker research and index-level attribution.

**Chat window regressions reported by the user:**

6. **Redundant Brave Search integration.** An earlier Round 4 draft wired a
   Brave Search API fetcher into `research.rs` so the AI could augment the
   packet with web results. The user flagged this as redundant — Claude and
   Gemini CLIs already perform live web searches when asked. The Brave code
   was dead weight and a new API dependency for zero net benefit.
7. **Vertical expansion regression.** The Claude Code chat window's scroll
   area was hardcoded to `.max_height(340.0)` (and the AI Assistant window to
   `.max_height(300.0)`), so resizing the window vertically left a wasted
   bottom gap. Gemini had been fixed earlier — Claude and the AI Assistant
   had regressed to the same pattern.
8. **Session memory loss.** Each Send in the Claude Code window spawned a
   fresh `claude --print <msg>` subprocess with no prior context, so
   follow-ups ("what did you mean about capex?") were answered as if the
   assistant had never seen the research packet. The user had to re-send the
   full packet every time, which defeated the window's purpose.
9. **Web tool permission prompts denied.** Claude Code running in `--print`
   mode cannot show interactive permission prompts — so `WebSearch` and
   `WebFetch` were silently refused every time the model tried to use them.
   The user reported "Claude said permissions were denied" with no way to
   approve them.
10. **No model picker.** The user wanted to explicitly run Claude Opus at
    maximum effort for the hardest research questions; Gemini users wanted
    to choose between Pro and Flash. No UI existed for this.

The user's bar remains "rival TradingView; TradingView was inferior to
Godel." Round 4 plus the chat window overhaul together close the last known
research-surface gaps and make the AI integrations usable for sustained
multi-turn research sessions.

## Decision

Add five new Godel-parity windows following the exact ADR-107/108/109/110
pattern: typed data → research module fetcher → `BrokerCmd`/`BrokerMsg`
pair → SQLite cache + LAN sync whitelist → egui window render → command
palette entries → bulk-scrape integration. Separately, rebuild the Claude
Code and Gemini CLI chat windows to solve the session-continuity,
permission-model, and layout regressions together — a single end-to-end
rewrite is simpler than six separate patches.

### 1. New types in `engine/src/core/research.rs`

```rust
pub struct StockSplit {
    pub date: String, pub label: String,
    pub numerator: f64, pub denominator: f64,
}

pub struct EtfHolding {
    pub symbol: String, pub name: String,
    pub weight_pct: f64, pub shares: f64,
    pub market_value: f64, pub updated: String,
}

pub struct AnalystRecommendation {
    pub period: String, pub strong_buy: i32,
    pub buy: i32, pub hold: i32,
    pub sell: i32, pub strong_sell: i32,
}

pub struct PriceTarget {
    pub symbol: String, pub target_high: f64,
    pub target_low: f64, pub target_mean: f64,
    pub target_median: f64, pub last_updated: String,
    pub num_analysts: i32,
}

pub struct EsgScore {
    pub symbol: String, pub environmental_score: f64,
    pub social_score: f64, pub governance_score: f64,
    pub esg_score: f64, pub year: i32,
}

pub struct IndexMember {
    pub index: String, pub symbol: String, pub name: String,
    pub sector: String, pub sub_sector: String,
    pub headquarters: String, pub date_added: String,
}
```

All `#[derive(Default, Serialize, Deserialize)]` — identical roundtrip story
to Round 1-3 types. Research module type count goes from 13 to 19.

### 2. New fetchers

| Fn | Endpoint | Free-tier | Notes |
|---|---|---|---|
| `fetch_fmp_stock_splits` | `/api/v3/historical-price-full/stock_split/{sym}` | 250/day | Label already formatted (e.g. "2:1") |
| `fetch_fmp_etf_holdings` | `/api/v3/etf-holder/{sym}` | 250/day | Up to 1000 constituents |
| `fetch_finnhub_recommendations` | `/api/v1/stock/recommendation` | 60/min | Monthly buckets for ~12 months |
| `fetch_finnhub_price_target` | `/api/v1/stock/price-target` | 60/min | Single snapshot row |
| `fetch_fmp_esg` | `/api/v4/esg-environmental-social-governance-data?symbol={sym}` | 250/day | Historical year-over-year rows |
| `fetch_fmp_index_members` | `/api/v3/{sp500,nasdaq,dowjones}_constituent` | 250/day | Global (per-index, not per-symbol) |

All fetchers follow the established pattern: `reqwest::Client` + JSON parse
+ typed struct conversion + `unwrap_or_default()` for missing fields.

### 3. SQLite schema (`create_research_tables_v4`)

```sql
CREATE TABLE IF NOT EXISTS research_stock_splits (
    symbol TEXT PRIMARY KEY, rows_json TEXT DEFAULT '[]',
    updated_at INTEGER DEFAULT 0);
CREATE TABLE IF NOT EXISTS research_etf_holdings (
    symbol TEXT PRIMARY KEY, rows_json TEXT DEFAULT '[]',
    updated_at INTEGER DEFAULT 0);
CREATE TABLE IF NOT EXISTS research_analyst_recs (
    symbol TEXT PRIMARY KEY, rows_json TEXT DEFAULT '[]',
    updated_at INTEGER DEFAULT 0);
CREATE TABLE IF NOT EXISTS research_price_target (
    symbol TEXT PRIMARY KEY, target_json TEXT DEFAULT '{}',
    updated_at INTEGER DEFAULT 0);
CREATE TABLE IF NOT EXISTS research_esg (
    symbol TEXT PRIMARY KEY, rows_json TEXT DEFAULT '[]',
    updated_at INTEGER DEFAULT 0);
CREATE TABLE IF NOT EXISTS research_index_members (
    index_code TEXT PRIMARY KEY, rows_json TEXT DEFAULT '[]',
    updated_at INTEGER DEFAULT 0);
```

Plus one `idx_research_*_updated` BTREE per table for LAN-sync incremental
pulls. `research_price_target` uses a single-row `target_json` blob (since
the struct is atomic); the others use the standard `rows_json` vector blob
shape. `research_index_members` is keyed by `index_code` (not symbol) —
membership is a property of the index, not the ticker.

### 4. `BrokerCmd` / `BrokerMsg`

```rust
// cmd
FetchStockSplits         { symbol: String, fmp_key: String },
FetchEtfHoldings         { symbol: String, fmp_key: String },
FetchAnalystRecs         { symbol: String, finnhub_key: String },
FetchPriceTarget         { symbol: String, finnhub_key: String },
FetchEsgScore            { symbol: String, fmp_key: String },
FetchIndexMembers        { index_code: String, fmp_key: String },

// msg
StockSplitsMsg(String, Vec<StockSplit>),
EtfHoldingsMsg(String, Vec<EtfHolding>),
AnalystRecsMsg(String, Vec<AnalystRecommendation>),
PriceTargetMsg(String, PriceTarget),
EsgScoreMsg(String, Vec<EsgScore>),
IndexMembersMsg(String, Vec<IndexMember>),
```

Handlers reuse the ADR-107 async/sync split — `tokio::spawn` the fetch, emit
the `BrokerMsg`, the `update()` loop both updates in-memory state and calls
the sync upsert helper under `cache.connection()`.

### 5. UI — five new egui windows (COT deferred to paid-API phase)

| Window | Size | Layout |
|---|---|---|
| **SPLT** | 540×380 | Top bar + 3-column grid: Date / Ratio / num:den. Small window — most tickers have 0-3 splits. |
| **ETF** | 820×540 | Top bar + aggregate weight header + 5-column grid: Symbol / Name / Weight % / Shares / Market Value. Body scrollable — constituents can run to 1000 rows. |
| **ANR** | 700×460 | Top bar + price target consensus header (mean / median / range / analyst count) + 6-column rec grid: Period / SBuy / Buy / Hold / Sell / SSell. |
| **ESG** | 620×400 | Top bar + latest-composite header + 5-column grid: Year / Env / Soc / Gov / Composite. |
| **MEMB** | 880×560 | Top bar with index-code picker (SP500/NDX/DJIA radio) + member count header + 6-column grid: Symbol / Name / Sector / Sub-sector / HQ / Date Added. |

Each window mirrors the top-bar layout from ADR-108/109/110 — Symbol input /
Use Chart / Load Cached / Fetch / Status. The SPLT/ETF/ANR/ESG windows are
per-symbol; MEMB replaces the symbol controls with an index picker (the only
per-symbol concept for MEMB is "which index are we looking at").

### 6. Command palette entries

Added to the string-match dispatcher after the existing Round 3 arms:

```
SPLT  | SPLITS | SPLIT_HISTORY        → open SPLT, fetch splits
ETF   | ETF_HOLDINGS | HOLDINGS       → open ETF, fetch holdings
ANR   | ANALYST_RATINGS | RECS        → open ANR, fetch recs + PT
ESG   | ESG_SCORE | SUSTAINABILITY    → open ESG, fetch scores
MEMB  | INDEX_MEMBERS | CONSTITUENTS  → open MEMB, fetch SP500 by default
```

### 7. Bulk scrape integration

`scrape_and_cache_symbol` gets five new calls inserted after the existing
Round 3 block:

- FMP block adds: `fetch_fmp_stock_splits`, `fetch_fmp_etf_holdings`,
  `fetch_fmp_esg` — each followed by the standard 400 ms cooldown.
- Finnhub block adds: `fetch_finnhub_recommendations`,
  `fetch_finnhub_price_target` — each followed by a 1100 ms cooldown
  matching the other Finnhub-rate-limited calls.
- `fetch_fmp_index_members` is **not** part of the per-symbol sweep — it's
  a global single call that would be wasteful to run once per ticker. A
  future `MACRO_SCRAPE` or similar one-shot command is a better home.

Per-symbol sweep cost: +3 FMP calls × 400 ms + 2 Finnhub calls × 1100 ms =
**+3.4 s per symbol**. A 500-ticker sweep pushes wall time from ~65 min
(ADR-110) to ~93 min. Acceptable; bulk scrape is overnight work.

### 8. LAN sync whitelist

`engine/src/core/lan_sync.rs::SYNCABLE_TABLES` gains six new entries:
`research_stock_splits`, `research_etf_holdings`, `research_analyst_recs`,
`research_price_target`, `research_esg`, `research_index_members`.
`create_table_sql` gains matching `CREATE TABLE IF NOT EXISTS` clauses so a
fresh client can materialize the schema before its first sync pull.
`table_timestamp_column` maps all six to `updated_at`.

### 9. `investigate_symbols()` expansion

The research packet builder is extended to emit **nine new sub-blocks** per
symbol from the ADR-107/108/109/110/111 cached data:

- **Recent news** — 8 articles via `news::get_news_by_symbol` (ADR-107)
- **Dividend history** — 6 rows from `research::get_dividends` (ADR-109)
- **Forward earnings estimates** — 4 periods from `research::get_earnings_estimates` (ADR-109)
- **Analyst rating changes** — 6 rows from `research::get_rating_changes` (ADR-109)
- **Annual income/cashflow/balance trends** — 4 periods each from `research::get_financials` (ADR-110)
- **Management** — 6 executives from `research::get_executives` (ADR-110)
- **Stock split history** — 4 rows from `research::get_stock_splits` (ADR-111)
- **Analyst consensus** — price target + latest rec bucket from `research::get_price_target` + `research::get_analyst_recs` (ADR-111)
- **ESG score** — latest year from `research::get_esg` (ADR-111)

Each sub-block is emitted only when its source has rows — silent skip
otherwise. Average packet size doubles from 3-6 KB to 6-12 KB per symbol.

### 10. AI chat window overhaul

The Brave Search integration is **removed entirely**. The `WebSearchResult`
struct, `fetch_brave_search` fetcher, `strip_html_tags` helper, and the
corresponding tests are deleted. The `BRAVE` palette entry and the
`brave_key` / `brave_search_*` state fields are removed from `TyphooNApp`.
Reason: Claude and Gemini CLIs already perform live web searches when
prompted, and the hosted providers (OpenAI / Grok / Perplexity) also have
native search tools. Brave was a redundant dependency.

The Claude Code, Gemini CLI, and AI Assistant chat windows are rebuilt with
four shared fixes:

**A. Dynamic scroll height.** The hardcoded `.max_height(340.0)` /
`.max_height(300.0)` on the `ScrollArea` is replaced with:

```rust
let scroll_h = (ui.available_height() - 60.0).max(120.0);
egui::ScrollArea::vertical().max_height(scroll_h).show(ui, ...);
```

This reserves 60 px for the bottom input row and lets the scroll area
expand to fill any remaining vertical space. Each window also gains
`min_width(420.0).min_height(280.0).constrain(true)` on the `egui::Window`
builder so it can't be collapsed to unusable sizes.

**B. Session continuity.** Each chat window stores the research packet in
its own `*_packet: Option<String>` field on `TyphooNApp`:

- `claude_code_packet: Option<String>`
- `gemini_cli_packet: Option<String>`
- `ai_chat_packet: Option<String>`

The ASKCLAUDE / ASKGEMINI / ASKAI handlers set this field when they dispatch
the initial query. Every subsequent Send in the window rebuilds the full
prompt from `packet + transcript + new message` via a new helper:

```rust
fn build_claude_prompt(
    packet: Option<&str>,
    history: &[(bool, String)],
    latest: &str,
) -> String
```

For the Claude Code window specifically, a per-window UUID v4 is generated
on first send and passed to `claude --session-id <uuid>`; subsequent sends
pass `--resume <uuid>`. The UUID is generated by a new LCG-based
`new_uuid()` helper (no new crate dependency). The packet injection is
belt-and-suspenders — even if the CLI loses its session state, the
re-injection on every Send guarantees the model sees the full context.

The AI Assistant window's `BrokerCmd::AiChat` variant gains two new fields:
`system: Option<String>` and `model: Option<String>`. The handler now
injects the packet into the Anthropic top-level `system` field (or the
OpenAI-compatible `{"role":"system"}` message) instead of pushing it as a
user turn. Max response tokens bumps from 1024 to 4096 across all
providers.

**C. Web tool permissions.** Claude Code is invoked with:

```sh
claude --print \
       --model <opus|sonnet|haiku> \
       --allowed-tools "WebSearch WebFetch Read Grep Glob Bash" \
       --permission-mode acceptEdits \
       [--session-id <uuid> | --resume <uuid>] \
       <prompt>
```

`--allowed-tools` pre-grants the read-only tools the CLI would otherwise
refuse in non-interactive `--print` mode. `--permission-mode acceptEdits`
silences the edit-confirmation prompt for Bash and file-writing tools. The
list is intentionally conservative — no `Write`, no `Edit`, no destructive
Bash patterns — since the CLI can't gate them with an interactive approval
in this mode.

**D. Model picker.** Both Claude Code and Gemini CLI windows gain an
`egui::ComboBox` in the top bar:

- Claude: `opus` (default — "max effort"), `sonnet` (balanced), `haiku` (fast)
- Gemini: `gemini-2.5-pro` (default), `gemini-2.5-flash`, `gemini-2.0-flash`
- AI Assistant: dynamically populated from the currently-selected provider

The picked model is passed as `--model <slug>` to the CLI, or as the
`"model"` field in the HTTP body for the AI Assistant window. Default
stored in `claude_model: String = "opus"` and `gemini_model: String =
"gemini-2.5-pro"`.

## Alternatives considered

- **Keep Brave Search as a "RESEARCH_PACKET" augmentation step.** Rejected
  — the CLIs already do this and better. Every kept Brave dependency would
  have been maintenance cost for zero user-visible benefit, and a moderate
  attack surface (API key persisted in keyring, fetch path on the terminal).
- **Split `research_price_target` into a per-analyst table instead of a
  single-row blob.** Rejected — Finnhub's free tier only returns the
  aggregate row anyway. Paid tiers with per-analyst detail could split
  later.
- **Use `gh` style one-shot pipes for Claude instead of `--session-id`.**
  Rejected — the user explicitly reported the "lost context on follow-up"
  regression, which is exactly what one-shot piping produces. Session state
  is the fix for the exact problem.
- **Add `FetchAllResearch` as a single `BrokerCmd` that fan-outs Round 1-4
  fetchers.** Deferred — would simplify the UI "just give me everything"
  flow but doesn't unlock anything the bulk scrape doesn't already cover.
  A good v2 ergonomic win.
- **Bundle the Brave Search removal into a separate cleanup commit.**
  Rejected — the removal is logically part of the chat window rework (the
  motivation for removing it came from the same user feedback pass). One
  atomic change is simpler to reason about and revert.
- **Wire COT as a sixth window in Round 4.** Deferred — ADR-110 already
  designed the CFTC public-endpoint pattern but deferred it as "not urgent".
  It remains a good near-term win but didn't make the Round 4 cut.

## Consequences

**Positive:**

- Five more Godel-parity surfaces land — `core/research.rs` now owns **19
  research types** (was 13) with a single test suite and a single bulk
  scrape entry point.
- The research packet roughly doubles its data density (3-6 KB → 6-12 KB
  per symbol). AI responses get materially better grounding because the
  model now starts with fundamentals, news, dividends, analyst consensus,
  financial trajectory, management, and ESG instead of just valuation
  ratios + SEC filings.
- The Claude Code chat window is finally usable as a sustained research
  conversation tool. Follow-ups see the full packet + transcript. Web
  search works. Vertical resize works. The user can explicitly pick Opus
  when they need maximum effort.
- Brave Search removal eliminates one API dependency, one crate of
  maintenance surface, and four tests for a feature the CLIs already
  provided natively.
- LAN sync whitelist now covers 21 tables (was 15), keeping the
  multi-terminal story intact.
- Research module test count: **26 passing** (was 17). New tests cover
  SPLT/ETF/ANR/PT/ESG/MEMB roundtrips through their upsert/get helpers.

**Trade-offs:**

- Per-symbol bulk scrape wall time grows from ~65 min to ~93 min for a
  500-ticker run. Still overnight-work tolerable, but the per-symbol cost
  is no longer dominated by Finnhub — the FMP 400 ms cooldowns are now
  the long pole. Could parallelize providers in a future pass.
- Packet size doubling means more tokens per AI call. For paid APIs
  (Claude / OpenAI / Perplexity) this is a linear cost increase. For the
  CLI path (ASKCLAUDE / ASKGEMINI) it costs nothing beyond a slight
  increase in prompt processing time.
- The `--allowed-tools "WebSearch WebFetch Read Grep Glob Bash"` flag
  intentionally excludes `Write` and `Edit`, so users asking Claude to
  "modify this file" via the ASKCLAUDE window will see the model refuse
  to write. That's the right default for a research surface, but a future
  "dev mode" toggle could relax it.
- Per-window session UUIDs are held in memory only — a TyphooN restart
  loses the Claude CLI resume state. Not worth persisting: the research
  packet re-injection gives the model everything it needs to continue the
  thread from scratch, so the CLI session ID is just a small optimization
  for the first turn after reload.
- ETF window shows a single flat list for 1000-constituent ETFs like VTI —
  no sector rollup or top-N condense. Acceptable v1; a future pivot to a
  sector-aggregated view would be a UX win.
- The AI Assistant window's model picker is position-locked to the
  currently-selected provider — changing provider resets the model to the
  provider default. A per-provider "remember last model" map could smooth
  this.

## Tests

9 new unit tests in `core::research::tests`:

- `stock_split_roundtrip` / `stock_split_default_is_empty`
- `etf_holding_roundtrip`
- `analyst_rec_roundtrip`
- `price_target_roundtrip` / `price_target_default_is_empty` / `price_target_upsert_replaces`
- `esg_roundtrip`
- `index_member_roundtrip`

Engine research module test count: **26 passing** (was 17). Full engine
suite: **598 passing**.

## Related

- ADR-107 — Multi-source news ingest (async/sync split pattern)
- ADR-108 — Godel parity round 1 (DES/PEERS/ERN/PRESS/SENTIMENT/TRANSCRIPTS/GLCO/IPO/TAS)
- ADR-109 — Godel parity round 2 (DVD/EEB/UPDG/GY)
- ADR-110 — Godel parity round 3 (FA/MGMT/COT deferred)
- `engine/src/core/research.rs` — fetchers, types, SQLite helpers
- `engine/src/core/lan_sync.rs::SYNCABLE_TABLES` — LAN sync whitelist
- `native/src/app.rs::investigate_symbols()` — research packet builder
- `native/src/app.rs::build_claude_prompt()` — subprocess prompt assembler
- `native/src/app.rs::new_uuid()` — Claude CLI session UUID generator
- `docs/RESEARCH_PACKET.md` — packet schema reference
