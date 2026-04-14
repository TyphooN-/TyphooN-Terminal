# TyphooN Terminal — Research Packet

The **research packet** is the markdown-formatted context block TyphooN assembles
whenever the user asks an AI model about one or more trading symbols. It is the
single payload that crosses the wire from the terminal to every supported AI
backend — Claude (API + CLI), GPT, Gemini (API + CLI), Grok, Mistral, Perplexity,
and local Ollama / LM Studio.

The packet exists so the LLM starts with a complete TyphooN-native view of the
ticker before it reaches for its own tools. Every field is pulled from the
terminal's own SQLite cache or in-memory broker state, snapped at the moment
the user issues the command, and dropped into a markdown document the model
reads verbatim. What you see in the packet is what the model sees.

Unlike earlier versions, the packet now **combines** with live web search
performed by Claude / Gemini CLIs (or the hosted provider's search tool) —
the system prompt explicitly instructs the model to cross-reference the packet
with real-time news, prices, and sentiment when the question calls for it.

> Source of truth: `native/src/app.rs::investigate_symbols()`

---

## Triggers

Three console commands build a research packet and dispatch it. All three
share the same argument parser (`parse_ask_args`) and the same packet builder
(`investigate_symbols`) — they only differ in how the built packet is
delivered to the model.

| Command | Transport | Destination |
|---|---|---|
| `ASKAI SYM[,SYM] [question]` | HTTP `POST` via `BrokerCmd::AiChat` | Currently-selected AI provider (AI Assistant window) |
| `ASKCLAUDE SYM[,SYM] [question]` | `claude --print` subprocess | Anthropic's `claude` CLI (must be on `$PATH`) |
| `ASKGEMINI SYM[,SYM] [question]` | `gemini --prompt` subprocess | Google's `gemini` CLI (must be on `$PATH`) |

Argument parsing contract: the first whitespace-separated token is the
comma-separated symbol list; everything after the first whitespace is the
question, preserved verbatim. Aliases: `ASK_AI`, `ASK_CLAUDE`, `ASK_GEMINI`,
and `INVESTIGATE` (for ASKAI).

Examples:

```
ASKAI CC                                    → packet for CC, default question
ASKAI CC,NCLH                               → packet for CC + NCLH, default question
ASKAI CC what is the debt load?             → packet for CC, custom question
ASKCLAUDE AAPL,MSFT,NVDA write me a memo    → 3-symbol packet to the Claude CLI
```

---

## Packet Layout

The packet is a single UTF-8 markdown string with one **header block**, one
**per-symbol section** per requested symbol separated by `---`, and a
**closing question block**.

### 1. Header

```markdown
# TyphooN Terminal Research Packet
Scope: <broker scope label> | Generated: 2026-04-14T14:22:07Z
Symbols: CC, NCLH
```

- **Scope** comes from `self.broker_scope_label()` — reflects which brokers
  (MT5, Alpaca, TastyTrade) were active when the packet was built.
- **Generated** is a UTC ISO-8601 timestamp taken at packet-build time.
- **Symbols** is the joined list the user passed.

### 2. Per-symbol section

Each symbol is preceded by `---` and an `## {SYMBOL}` heading. Sections are
emitted in the order the user specified them. A section is composed of up to
**sixteen sub-blocks**, each of which is skipped silently when its data source
is empty.

#### 2.1 Company header + description

```markdown
**Carnival Corp** — Consumer Cyclical / Travel Services
<description up to 800 chars>
```

Pulled from `self.bg.all_fundamentals` — the same cache that populates the
EVSCRAPE / fundamentals window. `(unnamed)` / `Unknown` are emitted as
placeholders when the cache row has empty strings. The description is
hard-truncated to **800 characters** to keep the packet bounded.

If no fundamentals row exists for the symbol, the sub-block degrades to the
line:

```markdown
_No fundamentals on file for this symbol. Run EVSCRAPE to populate._
```

Every later sub-block continues to emit as long as its own source has data.

#### 2.2 Valuation & Risk table

A 20-row markdown table pulled from the same `Fundamentals` row: Market Cap,
Enterprise Value, MCap/EV %, Total Debt, Cash & Equivalents, Stock Price, P/E,
Forward P/E, PEG, P/B, P/S, EV/EBITDA, Profit Margin, Operating Margin, ROE,
ROA, Beta, Short Ratio, Short % of Float, Dividend Yield, Next Earnings.

Formatters: money values use `format_large_number` (`1.23B`, `456.7M`),
ratios use 2-decimal fixed, missing values render as `—`.

#### 2.3 Quarterly financials

Pulled from SQLite via `fundamentals::get_quarterly_financials`. Capped at
**4 quarters** — most recent first. Columns: Period / Revenue / Net Income /
FCF / Gross Profit / Op Income / EPS.

#### 2.4 Top institutional holders

Pulled from SQLite via `get_institutional_holders`. Capped at **5 rows**.
Columns: Holder / Shares / % Held / Value.

#### 2.5 Recent SEC filings

Filtered from `self.bg.sec_filings` by ticker. Capped at **10 filings**.
Each summary is truncated to **120 characters** to keep row lengths
predictable for LLM tokenization.

#### 2.6 Insider activity

Pulled from `self.bg.insider_trades`. Emits two aggregate lines — total
counts and buy/sell/net dollar values — followed by the **5 most recent
trades**. Form-4 transaction codes `P` and `S` are treated as buy and sell.

#### 2.7 Price & volatility

Source: daily OHLCV bars from the bar cache. Key probed in this order:

1. `mt5:CC:{sym}:1Day` — MT5 corporate-action-adjusted
2. `mt5:{sym}:1Day`    — MT5 raw
3. `alpaca:{sym}:1Day` — Alpaca daily bars

The first key with **≥20 bars** wins. Emitted metrics: last close, 20d / 60d /
252d returns, ATR(14) (Wilder-smoothed), VaR 95% (from
`typhoon_engine::core::var::compute_var_from_closes`).

#### 2.8 Recent news

New in ADR-111 — pulled from the multi-source news pipeline (ADR-107) via
`typhoon_engine::core::news::get_news_by_symbol`. Capped at **8 articles**.
Columns: Date / Source / Sentiment / Headline.

Sentiment is whatever the upstream provider supplied (Marketaux, AlphaVantage,
FMP, Finnhub) — if empty it renders as `—`. The model is explicitly told to
augment this with a live web search in the system prompt.

#### 2.9 Dividend history (DVD)

Pulled from `research::get_dividends`. Capped at **6 rows**. Columns:
Ex-Date / Pay Date / Amount / Label. Source: ADR-109 DVD Godel window.

#### 2.10 Forward earnings estimates (EEB)

Pulled from `research::get_earnings_estimates`. Capped at **4 future
periods**. Columns: Period / EPS Avg / EPS Lo/Hi / Rev Avg / Analyst counts.
Source: ADR-109 EEB window.

#### 2.11 Analyst rating changes (UPDG)

Pulled from `research::get_rating_changes`. Capped at **6 most recent
changes**. Columns: Date / Firm / Action / From → To / Price Target.
Source: ADR-109 UPDG window.

#### 2.12 Annual financial statements trend (FA)

Pulled from `research::get_financials`. Three sub-tables, each capped at
**4 annual periods**:

- Income statement trend: FY / Revenue / Gross / Op Inc / Net Inc / EPS
- Cash flow trend: FY / CFO / Capex / FCF / Div Paid / Buybacks
- Balance sheet trend: FY / Total Assets / Net Debt / Total Equity

Source: ADR-110 FA window.

#### 2.13 Management (MGMT)

Pulled from `research::get_executives`. Capped at **6 executives**. Emits
the total compensation across all listed officers in the section header,
then a table of Name / Position / Since / Compensation. Source: ADR-110 MGMT
window.

#### 2.14 Stock split history (SPLT)

Pulled from `research::get_stock_splits`. Capped at **4 most recent splits**.
Columns: Date / Ratio. Source: ADR-111 SPLT window.

#### 2.15 Analyst consensus (ANR)

Combines `research::get_price_target` and `research::get_analyst_recs` into
a single block:

- Price target line: mean / median / range across all contributing analysts
- Rating breakdown line (latest period): Strong Buy / Buy / Hold / Sell / Strong Sell

Source: ADR-111 ANR window.

#### 2.16 ESG score

Pulled from `research::get_esg`. Shows latest-year environmental, social,
governance, and composite scores. Source: ADR-111 ESG window.

#### 2.17 Sector peer comparison

Emitted only when the fundamentals row has a non-empty sector AND at least
**3 other symbols** in `self.bg.all_fundamentals` share that sector. Compares
this symbol's P/E, Forward P/E, P/B, P/S, EV/EBITDA, Profit Margin, ROE,
Beta, Short % of Float, and Dividend Yield against the sector median.

### 3. Closing question

```markdown
---
## Question
<user question verbatim, OR default rubric when question is empty>
```

Default rubric (when the user issues `ASKAI SYM` with no trailing question):

> Using only the data above, write a concise investment research note on
> each symbol covering: (1) valuation vs sector peers, (2) financial
> trajectory from the quarterly data, (3) balance-sheet / solvency notes,
> (4) SEC filing activity and insider sentiment, (5) volatility regime and
> risk profile, and (6) a neutral-to-directional takeaway. Flag any data
> gaps you'd want filled in to refine the view.

---

## Size caps (hard limits in the builder)

| Field | Cap | Why |
|---|---|---|
| Company description | 800 chars | Some 10-K descriptions run thousands of chars |
| SEC filing summary | 120 chars | Keeps table rows readable |
| Quarterly financials | 4 rows | Model only needs a trajectory, not a decade |
| Institutional holders | 5 rows | Top-5 captures >50% of float for most names |
| Recent SEC filings | 10 rows | Covers last ~2 years for an active issuer |
| Insider trades | 5 rows | Aggregate values already cover the summary |
| Recent news | 8 articles | Matches the news window's top slice |
| Dividend history | 6 rows | Multi-year cadence visible in 6 |
| Earnings estimates | 4 periods | Forward 1Y coverage |
| Rating changes | 6 rows | Recent analyst rotation |
| Annual statements (I/B/C) | 4 periods each | 4-year trajectory |
| Management | 6 execs | Named officers typically ≤6 |
| Stock splits | 4 rows | Historical splits rarely exceed 4 |
| Daily bars required for stats | ≥20 | Needed for 20d return and ATR warm-up |

There is no global packet size limit — total size scales with the number of
symbols. A single S&P 500 symbol now produces a packet around **6-12 KB**
(up from 3-6 KB before the ADR-111 expansion); a 10-symbol basket lands
near **60-120 KB**.

---

## AI provider wire formats

The packet is delivered via two code paths: HTTP (hosted APIs) or subprocess
(local CLIs).

### HTTP path (ASKAI → AI Assistant window)

`BrokerCmd::AiChat` now takes `system: Option<String>` and
`model: Option<String>` fields. The research packet is **injected as the
system prompt** — not as a user turn — along with a trading-assistant
preamble that explicitly asks the model to combine the packet with live web
search for news / sentiment / prices.

The AI Assistant window shows a `[packet loaded]` indicator when a packet
is in scope, and a model picker ComboBox that resets to the provider default
when the provider changes.

**Anthropic** (native API format, `system` is a top-level field):

```http
POST https://api.anthropic.com/v1/messages
x-api-key: <anthropic_key>
anthropic-version: 2023-06-01
content-type: application/json

{
  "model": "claude-opus-4-5",
  "max_tokens": 4096,
  "system": "<preamble + RESEARCH PACKET block>",
  "messages": [...history..., {"role": "user", "content": "<latest turn>"}]
}
```

**OpenAI-compatible path** — used for the remaining six providers. The full
system prompt (preamble + packet) is prepended as a `{"role":"system"}`
message.

| Provider | URL | Default Model |
|---|---|---|
| OpenAI | `https://api.openai.com/v1/chat/completions` | `gpt-4o` |
| Google Gemini | `https://generativelanguage.googleapis.com/v1beta/openai/chat/completions` | `gemini-2.5-pro` |
| xAI / Grok | `https://api.x.ai/v1/chat/completions` | `grok-3` |
| Mistral | `https://api.mistral.ai/v1/chat/completions` | `mistral-large-latest` |
| Perplexity | `https://api.perplexity.ai/chat/completions` | `sonar-pro` |
| Local (Ollama) | `http://localhost:11434/v1/chat/completions` | `llama3.2` |
| Local (LM Studio) | `http://localhost:1234/v1/chat/completions` | `llama3.2` |

Response text is extracted from `choices[0].message.content`. The local path
sends no `Authorization` header; the other five send `Bearer <api_key>`.
All providers use a **4096**-token response budget (up from 1024).

### Subprocess path (ASKCLAUDE / ASKGEMINI)

No network hop from the terminal. The packet, conversation transcript, and
latest user turn are rebuilt into a single prompt string via
`Self::build_claude_prompt()` every time the user clicks Send — so
follow-ups always see the full context without relying on fragile CLI state.

**Claude Code CLI** (`ASKCLAUDE` / Claude Code chat window):

```sh
claude --print \
       --model <opus|sonnet|haiku> \
       --allowed-tools "WebSearch WebFetch Read Grep Glob Bash" \
       --permission-mode acceptEdits \
       --session-id <uuid>  # first call
       # or
       --resume <uuid>      # subsequent calls in the same window
       "<full prompt string>"
```

- `--session-id` / `--resume` — each chat window holds a per-window v4 UUID
  that is passed to `--session-id` on the first send and to `--resume` on
  every subsequent send, so the CLI's own session state mirrors the
  in-window transcript.
- `--allowed-tools` — pre-grants `WebSearch` and `WebFetch` so live web
  search works inside `--print` mode (where the CLI cannot show interactive
  permission prompts). Read / Grep / Glob / Bash are added so the CLI can
  introspect TyphooN's source when the user asks.
- `--permission-mode acceptEdits` — silences the interactive edit
  confirmation that would otherwise block non-TTY invocations.
- `--model` — wired to the model picker in the chat window (default `opus`
  for maximum effort).

**Gemini CLI** (`ASKGEMINI` / Gemini CLI chat window):

```sh
gemini --model <gemini-2.5-pro|gemini-2.5-flash|gemini-2.0-flash> \
       --prompt "<full prompt string>"
```

Both handlers first run `which claude` / `which gemini`; if the binary is
missing, the command logs an error and the packet is never built. Each
subprocess runs on a dedicated `std::thread` so the UI stays responsive, and
the reply is piped back via a `std::sync::mpsc::channel` drained on the next
UI frame into the respective chat window.

### Prompt builder

`native/src/app.rs::build_claude_prompt(packet, history, latest)` assembles
the full prompt string for both subprocess paths:

```
=== RESEARCH PACKET ===
<packet>
=== END RESEARCH PACKET ===

=== PRIOR CONVERSATION ===
User: ...
Assistant: ...
=== END PRIOR CONVERSATION ===

User: <latest turn>
```

Prior `[Research packet: SYM]` placeholder entries are filtered out of the
transcript so the model doesn't see duplicated meta-labels.

### Session continuity

Each chat window (Claude Code, Gemini CLI, AI Assistant) stores the packet
in its own `*_packet: Option<String>` field. Every Send rebuilds the prompt
from the stored packet + the transcript + the new message, so the model
never "forgets" what TyphooN handed it — even if the CLI itself would
otherwise treat each `--print` invocation as a fresh conversation.

---

## Data sources referenced by the builder

| Source | Kind | Populated by |
|---|---|---|
| `self.bg.all_fundamentals` | `Vec<Fundamentals>` | EVSCRAPE / `FundamentalsScrape` |
| `self.bg.sec_filings` | `Vec<SecFiling>` | SEC filings window / scraper (ADR-096) |
| `self.bg.insider_trades` | `HashMap<String, Vec<InsiderTrade>>` | Insider trades fetcher |
| `fundamentals::get_quarterly_financials` | SQLite `quarterly_financials` | `fundamentals` module |
| `fundamentals::get_institutional_holders` | SQLite `institutional_holders` | `fundamentals` module |
| `news::get_news_by_symbol` | SQLite `research_news` + FTS5 | ADR-107 news pipeline |
| `research::get_dividends` | SQLite `research_dividends` | ADR-109 DVD window |
| `research::get_earnings_estimates` | SQLite `research_earnings_estimates` | ADR-109 EEB window |
| `research::get_rating_changes` | SQLite `research_rating_changes` | ADR-109 UPDG window |
| `research::get_financials` | SQLite `research_financials` | ADR-110 FA window |
| `research::get_executives` | SQLite `research_executives` | ADR-110 MGMT window |
| `research::get_stock_splits` | SQLite `research_stock_splits` | ADR-111 SPLT window |
| `research::get_analyst_recs` | SQLite `research_analyst_recs` | ADR-111 ANR window |
| `research::get_price_target` | SQLite `research_price_target` | ADR-111 ANR window |
| `research::get_esg` | SQLite `research_esg` | ADR-111 ESG window |
| `cache.get_bars_raw` | SQLite bar cache | MT5SYNC, BARDATA, chart loads |
| `self.broker_scope_label()` | in-memory | active broker flags |

If a given source is empty, the corresponding sub-block is silently omitted
(or replaced with a "Run X to populate" hint for fundamentals and bars).

---

## Failure modes

- **No symbols parsed** — the window opens, the terminal logs
  `Usage: ASKAI SYM1[,SYM2] [optional question]`, no packet is sent.
- **Empty API key (HTTP path)** — the chat shows `Set API key in Settings
  first.`; the `BrokerCmd::AiChat` is never dispatched. The `local` provider
  has no key requirement.
- **CLI binary missing (subprocess path)** — the log shows
  `Claude Code CLI not found in PATH.` / `Gemini CLI not found in PATH.`.
- **Concurrent CLI invocations** — while a previous ASKCLAUDE / ASKGEMINI is
  still running, a new trigger is a no-op. The first reply must land before
  a second CLI call will fire.
- **Missing `--session-id` UUID** — if for any reason `claude_code_session_id`
  is empty, a fresh UUID is generated on the next Send.
- **Empty bar cache** — price & volatility sub-block is replaced with a
  "run MT5SYNC or BARDATA" hint; everything else still emits.

---

## Related

- `native/src/app.rs::investigate_symbols()` — the builder
- `native/src/app.rs::parse_ask_args()` — argument parser
- `native/src/app.rs::build_claude_prompt()` — prompt assembler for subprocess paths
- `native/src/app.rs::new_uuid()` — UUID v4 generator for session ids
- `docs/API_KEYS.md` — free-tier provider keys
- ADR-096 — SEC filing expansion
- ADR-107 — Multi-source news ingest
- ADR-108 / 109 / 110 / 111 — Godel parity research surfaces
