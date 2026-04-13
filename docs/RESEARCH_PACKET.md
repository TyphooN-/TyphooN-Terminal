# TyphooN Terminal — Research Packet

The **research packet** is the markdown-formatted context block TyphooN assembles
whenever the user asks an AI model about one or more trading symbols. It is the
single payload that crosses the wire from the terminal to every supported AI
backend — Claude, GPT, Gemini, Grok, Mistral, Perplexity, and local Ollama.

The packet exists so the LLM never has to call out to the internet, invent
numbers, or guess at stale training data. Every field is pulled from the
terminal's own SQLite cache or in-memory broker state, snapped at the moment
the user issues the command, and dropped into a markdown document the model
reads verbatim. What you see in the packet is what the model sees — no hidden
tools, no retrieval, no post-hoc injection.

> Source of truth: `native/src/app.rs::investigate_symbols()` (lines 15427-15679)

---

## Triggers

Three console commands build a research packet and dispatch it. All three
share the same argument parser (`parse_ask_args`) and the same packet builder
(`investigate_symbols`) — they only differ in how the built packet is
delivered to the model.

| Command | Transport | Destination |
|---|---|---|
| `ASKAI SYM[,SYM] [question]` | HTTP `POST` via `BrokerCmd::AiChat` | Currently-selected AI provider (Settings → AI Provider) |
| `ASKCLAUDE SYM[,SYM] [question]` | `claude --print <prompt>` subprocess | Anthropic's `claude` CLI (must be on `$PATH`) |
| `ASKGEMINI SYM[,SYM] [question]` | `gemini <prompt>` subprocess | Google's `gemini` CLI (must be on `$PATH`) |

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

Handlers: `native/src/app.rs:17770` (ASKAI), `:17806` (ASKCLAUDE), `:17855` (ASKGEMINI).

---

## Packet Layout

The packet is a single UTF-8 markdown string with one **header block**, one
**per-symbol section** per requested symbol separated by `---`, and a
**closing question block**.

### 1. Header

```markdown
# TyphooN Terminal Research Packet
Scope: <broker scope label> | Generated: 2026-04-13T14:22:07Z
Symbols: CC, NCLH
```

- **Scope** comes from `self.broker_scope_label()` — reflects which brokers
  (MT5, Alpaca, TastyTrade) were active when the packet was built.
- **Generated** is a UTC ISO-8601 timestamp taken at packet-build time.
- **Symbols** is the joined list the user passed.

### 2. Per-symbol section

Each symbol is preceded by `---` and an `## {SYMBOL}` heading. Sections are
emitted in the order the user specified them. A section is composed of up to
**seven sub-blocks**, each of which is skipped silently when its data source
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

A 20-row markdown table pulled from the same `Fundamentals` row:

| Metric | Value |
|---|---|
| Market Cap | … |
| Enterprise Value | … |
| MCap/EV % | … |
| Total Debt | … |
| Cash & Equivalents | … |
| Stock Price | … |
| P/E (trailing) | … |
| Forward P/E | … |
| PEG | … |
| P/B | … |
| P/S | … |
| EV/EBITDA | … |
| Profit Margin | … |
| Operating Margin | … |
| ROE | … |
| ROA | … |
| Beta | … |
| Short Ratio | … |
| Short % of Float | … |
| Dividend Yield | … |
| Next Earnings | … |

Formatters: money values use `format_large_number` (`1.23B`, `456.7M`),
ratios use 2-decimal fixed, missing values render as `—`.

#### 2.3 Quarterly financials

```markdown
### Last 4 Quarterly Financials
| Period | Revenue | Net Income | FCF | Gross Profit | Op Income | EPS |
```

Pulled from SQLite via
`typhoon_engine::core::fundamentals::get_quarterly_financials(&conn, sym)`.
Capped at **4 quarters** — most recent first.

#### 2.4 Top institutional holders

```markdown
### Top 5 Institutional Holders
| Holder | Shares | % Held | Value |
```

Pulled from SQLite via `get_institutional_holders`. Capped at **5 rows**.

#### 2.5 Recent SEC filings

```markdown
### Recent SEC Filings (N)
| Date | Form | Category | Summary |
```

Filtered from `self.bg.sec_filings` by ticker (case-insensitive). Capped at
**10 filings**. Each summary is truncated to **120 characters** to keep row
lengths predictable for LLM tokenization.

#### 2.6 Insider activity

```markdown
### Insider Activity
- 14 transactions on file (3 buys, 11 sells)
- Buy aggregate: 1.2M | Sell aggregate: 8.7M | Net: -7.5M
| Date | Insider | Title | Type | Shares | Value |
```

Pulled from `self.bg.insider_trades` (a `HashMap<String, Vec<InsiderTrade>>`).
Emits two aggregate lines — total counts and buy/sell/net dollar values —
followed by the **5 most recent trades**. "Buys" and "sells" are detected by
SEC form 4 transaction codes `P`/`S` plus loose substring match on the
transaction-type string.

#### 2.7 Price & volatility

```markdown
### Price & Volatility (D1 bars, n=252)
- Last close: **24.3100**
- 20d return: +3.82%
- 60d return: -12.41%
- 252d return: +47.15%
- ATR(14): 0.7834 (3.22% of price)
- VaR 95% (1 lot): $123.45 (2.17% of ask)
```

Source: daily OHLCV bars from the bar cache. Key probed in this order:

1. `mt5:CC:{sym}:1Day` — MT5 corporate-action-adjusted
2. `mt5:{sym}:1Day`    — MT5 raw
3. `alpaca:{sym}:1Day` — Alpaca daily bars

The first key with **≥20 bars** wins. If none qualifies:

```markdown
_No D1 bar data in cache — price/volatility stats unavailable. Run MT5SYNC or BARDATA to populate._
```

Calculations:

- **Returns** — simple close-to-close at 20, 60, 252 sessions (rendered as
  `—` when the series is shorter than the lookback)
- **ATR(14)** — Wilder-smoothed true range on the final 14 sessions
- **VaR 95%** — `typhoon_engine::core::var::compute_var_from_closes(&closes, 0.95)`,
  expressed as both dollar amount and ratio to ask price

#### 2.8 Sector peer comparison

```markdown
### Sector Peer Comparison (Consumer Cyclical — 42 peers)
| Metric | This Symbol | Sector Median |
```

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

The "Using only the data above" framing is deliberate — it discourages the
model from hallucinating numbers that aren't in the packet.

---

## Size caps (hard limits in the builder)

| Field | Cap | Why |
|---|---|---|
| Company description | 800 chars | Some 10-K-sourced descriptions run thousands of chars |
| SEC filing summary | 120 chars | Keeps table rows readable |
| Quarterly financials | 4 rows | Model only needs a trajectory, not a decade |
| Institutional holders | 5 rows | Top-5 captures >50% of float for most names |
| Recent SEC filings | 10 rows | Covers last ~2 years for an active issuer |
| Insider trades | 5 rows | Aggregate values already cover the summary |
| Daily bars required for stats | ≥20 | Needed for min 20d return and ATR warm-up |

There is no global packet size limit — total size scales with the number of
symbols and the per-symbol density. In practice a single S&P 500 symbol
produces a packet around **3-6 KB**; a 10-symbol basket lands near **30-60
KB**.

---

## AI provider wire formats

The packet is delivered to seven backends via two different code paths.

### HTTP path (ASKAI)

`BrokerCmd::AiChat` handler at `native/src/app.rs:11530`. The packet becomes
the final `user` message appended to the current chat history. The
`max_tokens` response budget is **1024** for every provider.

**Anthropic** (native API format — not OpenAI-compatible):

```http
POST https://api.anthropic.com/v1/messages
x-api-key: <anthropic_key>
anthropic-version: 2023-06-01
content-type: application/json

{
  "model": "claude-sonnet-4-20250514",
  "max_tokens": 1024,
  "messages": [...history..., {"role": "user", "content": "<research packet>"}]
}
```

Response text is extracted from `content[0].text`.

**OpenAI-compatible path** — used for the remaining six providers. An extra
system message is prepended: `"You are a trading assistant for TyphooN
Terminal."`

| Provider | URL | Model |
|---|---|---|
| OpenAI | `https://api.openai.com/v1/chat/completions` | `gpt-4o` |
| Google Gemini | `https://generativelanguage.googleapis.com/v1beta/openai/chat/completions` | `gemini-2.5-flash` |
| xAI / Grok | `https://api.x.ai/v1/chat/completions` | `grok-3-mini` |
| Mistral | `https://api.mistral.ai/v1/chat/completions` | `mistral-large-latest` |
| Perplexity | `https://api.perplexity.ai/chat/completions` | `sonar-pro` |
| Local (Ollama) | `http://localhost:11434/v1/chat/completions` | `llama3.2` |
| Local (LM Studio) | `http://localhost:1234/v1/chat/completions` | `llama3.2` |

Response text is extracted from `choices[0].message.content`. The local path
sends no `Authorization` header; the other five send `Bearer <api_key>`.

### Subprocess path (ASKCLAUDE / ASKGEMINI)

No network hop from the terminal. The packet is spawned directly into the
local CLI as a single command-line argument, and the process's stdout is
captured as the response.

```sh
claude --print "<research packet>"      # ASKCLAUDE
gemini          "<research packet>"      # ASKGEMINI
```

Both handlers first run `which claude` / `which gemini`; if the binary is
missing, the command logs an error and the packet is never built. The
subprocess runs on a dedicated `std::thread` so the UI stays responsive, and
the reply is piped back via a `std::sync::mpsc::channel` into
`self.claude_code_rx` / `self.gemini_cli_rx` and drained on the next UI
frame into the respective chat window.

---

## Data sources referenced by the builder

These are the in-memory / on-disk structures the builder reads to assemble
the packet. None of them are populated by `investigate_symbols` itself — it
is a pure read.

| Source | Kind | Populated by |
|---|---|---|
| `self.bg.all_fundamentals` | `Vec<Fundamentals>` | EVSCRAPE / `FundamentalsScrape` |
| `self.bg.sec_filings` | `Vec<SecFiling>` | SEC filings window / scraper (ADR-096) |
| `self.bg.insider_trades` | `HashMap<String, Vec<InsiderTrade>>` | Insider trades fetcher |
| `cache.get_quarterly_financials` | SQLite — `fundamentals_quarterly` | `fundamentals` module |
| `cache.get_institutional_holders` | SQLite — `institutional_holders` | `fundamentals` module |
| `cache.get_bars_raw` | SQLite — bar cache | MT5SYNC, BARDATA, chart loads |
| `self.broker_scope_label()` | in-memory | active broker flags |

If a given source is empty, the corresponding sub-block is either omitted or
replaced with a "Run X to populate" hint. This is by design — the AI should
see exactly what TyphooN has on hand so it can flag data gaps in its reply
rather than silently filling in blanks.

---

## Failure modes

- **No symbols parsed** — the window opens, the terminal logs
  `Usage: ASKAI SYM1[,SYM2] [optional question]`, no packet is sent.
- **Empty API key (HTTP path)** — the chat shows `Set API key in Settings
  first.`; the `BrokerCmd::AiChat` is never dispatched. Exception: the
  `local` provider has no key requirement.
- **CLI binary missing (subprocess path)** — the log shows
  `Claude Code CLI not found in PATH.` / `Gemini CLI not found in PATH.`.
- **Concurrent CLI invocations** — while a previous ASKCLAUDE / ASKGEMINI is
  still running (`claude_code_rx` / `gemini_cli_rx` still `Some`), a new
  trigger is a no-op. The first reply must land (or the channel must drop)
  before a second CLI call will fire.
- **Empty bar cache** — price & volatility sub-block is replaced with a
  "run MT5SYNC or BARDATA" hint; everything else still emits.

---

## Related

- `native/src/app.rs::investigate_symbols()` — the builder (lines 15427-15679)
- `native/src/app.rs::parse_ask_args()` — argument parser (lines 15681+)
- `native/src/app.rs` — `BrokerCmd::AiChat` handler (line 11530)
- `docs/API_KEYS.md` — free-tier provider keys (Anthropic, OpenAI, Gemini, …)
- ADR-096 — SEC filing expansion (source for filing sub-block)
- ADR-107 — Multi-source news ingest (companion context the AI does *not*
  currently receive; candidate for a future packet field)
