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

### 2. Global Market Context (ADR-113)

Emitted **once** at the top of the packet, before any per-symbol section.
Gives the model a regime-level view of risk-on/off, leadership/laggards, and
sector rotation at packet-build time. Entire section is skipped silently if
none of its three sources are cached.

#### Global.1 World Equity Indices (WEI)

Pulled from `research::get_world_indices`. Advancing/declining count line
followed by up to **12 indices** as a markdown table: region, ticker, name,
last, change %. Universe defined in `WORLD_INDICES_UNIVERSE` — spans 22
Americas / EMEA / Asia-Pacific indices sourced from Yahoo Finance (no API
key required). Populated by running the `WEI` command.

#### Global.2 Market Movers — US (MOV)

Pulled from `research::get_market_movers`. Three single-line aggregate
summaries for top gainers / losers / most active — each a comma-separated
list of up to **6 symbols** with their change %. Sourced from FMP's
`/v3/stock_market/{gainers,losers,actives}` endpoints. Populated by running
the `MOV` command.

#### Global.3 Sector Performance (INDU)

Pulled from `research::get_sector_performance`. Up/down aggregate counts plus
one line per S&P sector (sorted high-to-low on change %). Sourced from FMP's
`/v3/sector-performance` endpoint. Populated by running the `INDU` command.

#### Global.4 World Currency Rates (WCR — ADR-114)

Pulled from `research::get_currency_rates`. One header line (total pairs +
strengthening / weakening counts) then one summary line per region (Majors /
Crosses / EM) listing up to **8 pairs** with price and change %. Source:
Yahoo Finance `/v7/finance/quote` (no API key required). Universe defined
in `FX_MAJORS_UNIVERSE`. Populated by running the `WCR` command.

### 3. Per-symbol section

Each symbol is preceded by `---` and an `## {SYMBOL}` heading. Sections are
emitted in the order the user specified them. A section is composed of up to
**thirty-seven sub-blocks**, each of which is skipped silently when its data
source is empty.

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

#### 2.17 Insider flow (INS)

Pulled from `research::get_insider_trades`. Aggregate summary line
(`buys $X.XXM · sells $X.XXM · net $±X.XXM across N filings`) followed by
up to **8 most recent Form 4 rows**: date, BUY/SELL tag, insider name,
share count, price, dollar value, and transaction type code.
Source: ADR-112 INS window.

#### 2.18 Institutional holders (HDS)

Pulled from `research::get_institutional_holders`. Total holder count +
total shares in the section header, then up to **6 top holders** with
QoQ Δ (millions of shares) and report date. Source: ADR-112 HDS window.

#### 2.19 Shares float (FLOAT)

Pulled from `research::get_shares_float`. Single-line snapshot:
outstanding shares, float shares, free-float %, data source.
Source: ADR-112 FLOAT window.

#### 2.20 Recent price history (HP)

Pulled from `research::get_historical_price`. Emits a markdown table of
the **10 most recent daily bars**: Date / Open / High / Low / Close /
Volume / Change %. Analysts use this for quick audit trails when they
don't need to open the chart. Source: ADR-112 HP window.

#### 2.21 EPS surprise history (EPS)

Pulled from `research::get_earnings_surprises`. Aggregate summary line
(quarters tracked, beats, misses, 8-quarter rolling average surprise %)
followed by up to **8 most recent quarters**: date, actual EPS,
estimate EPS, surprise, surprise %. Source: ADR-112 EPS window.

#### 2.22 WACC snapshot (WACC — ADR-113)

Pulled from `research::get_wacc`. Three-line summary giving the CAPM-derived
cost of capital for the symbol: cost of equity + after-tax cost of debt +
weighted-average, then input lineage (β, Rf, ERP, tax rate), then capital
mix (equity/debt weights and absolute market-cap / debt figures). Skipped
silently when the WACC snapshot has not been computed (run the `WACC`
command to populate). Source: ADR-113 WACC window.

#### 2.23 Rolling beta vs SPY (BETA — ADR-114)

Pulled from `research::get_beta`. Markdown table of rolling-window β
observations (typically 1Y / 3Y / 5Y) with columns: Window / β / α (ann) /
R² / Corr / N. Computed via OLS on log-returns against SPY with date
intersection to guarantee matching observations. Populated by running the
`BETA` command, which fetches 5Y history for the symbol and SPY and computes
all windows in one shot. Source: ADR-114 BETA window.

#### 2.24 Gordon Growth DDM (DDM — ADR-114)

Pulled from `research::get_ddm`. Single summary line showing trailing annual
dividend (D0), implied growth g (from dividend CAGR), required return r
(ideally from WACC's cost of equity), and a bolded implied price line when
r > g. When r ≤ g, the block reports the caveat rather than a price.
Method: `P = D1 / (r - g)` where `D1 = D0 × (1 + g)`. Populated by running
the `DDM` command after dividend history is cached via the `DVD` window.
Source: ADR-114 DDM window.

#### 2.25 Relative valuation matrix (RV — ADR-114)

Pulled from `research::get_relative_valuation`. Markdown table of
peer-Z-score rows for P/E, Forward P/E, P/B, P/S, EV/EBITDA, Profit Margin,
ROE, Beta, and Dividend Yield. Columns: Metric / Value / Peer Median / Z /
Percentile. Peers are the symbol's sector peers from `research::get_peers`
(ADR-109 PEERS surface), and each metric is only emitted when there are ≥3
non-null peer values. Populated by running the `RV` command after peers
and fundamentals are cached. Source: ADR-114 RV window.

#### 2.26 Instrument identifiers (FIGI — ADR-114)

Pulled from `research::get_figi`. Up to **3 identifiers** per symbol, each
on its own line listing ticker, FIGI, share-class FIGI, exchange code, and
security description. Sourced from the free OpenFIGI `/v3/mapping` endpoint
(no API key required). Populated by running the `FIGI` command.
Source: ADR-114 FIGI window.

#### 2.27 Historical return / risk (HRA — ADR-115)

Pulled from `research::get_hra`. Two header lines giving annualized
volatility, Sharpe, Sortino, Calmar, risk-free rate used, and the
max-drawdown pair (peak-to-trough) — then a markdown table of rolling
window returns (1D / 5D / 1M / 3M / 6M / YTD / 1Y / 3Y / 5Y / ITD) with
Return / CAGR / N columns. Pure compute over cached daily bars from the
`HP` window; no fetcher. Populated by running the `HRA` command.
Source: ADR-115 HRA window.

#### 2.28 Discounted Cash Flow fair value (DCF — ADR-115)

Pulled from `research::get_dcf`. Four-line header with base TTM revenue /
FCFF / margin / growth / terminal growth / WACC / tax rate, PV of explicit
FCFF and terminal value separated, balance-sheet bridge (EV → equity value
via −debt +cash), and bolded implied price — followed by a **projection
year table** listing Revenue / EBIT / NOPAT / FCFF / PV FCFF for every
explicit-forecast year. Terminal value is Gordon growth
`TV = FCFF_N × (1+tg) / (WACC − tg)`; the block rejects configs where
`tg ≥ WACC − 0.5%` with a caveat note. Pure compute over
`fundamentals::get_quarterly_financials` (TTM roll-up) and `Fundamentals`
(balance-sheet items). Discount rate defaults to cached WACC when
available, 10% otherwise. Populated by running the `DCF` command.
Source: ADR-115 DCF window.

#### 2.29 Stock Valuation Model synthesis (SVM — ADR-115)

Pulled from `research::get_svm`. Two header lines giving current price,
fair-mid and upside %, plus low/high fair range — then a markdown table of
implied prices from every available model (WACC cost of equity, DDM Gordon
Growth, DCF FCFF, peer P/E × EPS, peer EV/EBITDA × EBITDA − debt + cash /
shares, peer P/B × BVPS) with columns: Model / Implied / Upside /
Confidence / Source. Triangulates DDM / DCF / peer multiples into a single
multi-anchor fair-value view. Pure compute; consumes cached DDM, DCF,
Fundamentals, and peer fundamentals. Populated by running the `SVM` command.
Source: ADR-115 SVM window.

#### 2.30 Options chain summary (OMON — ADR-115)

Pulled from `research::get_options_chain`. Three header lines giving
underlying price + cached expiration count, nearest-expiration DTE and
call/put counts, and an aggregate line with put/call volume ratio,
put/call open-interest ratio, ATM IV, and total call/put volume — followed
by an **ATM-zone chain table** showing up to **11 strikes** (5 below and 5
above the underlying, ATM in bold) with C Last / C IV / C Vol / C OI /
P Last / P IV / P Vol / P OI columns. Deep out-of-the-money / deep
in-the-money strikes stay in SQLite and the OMON window but are excluded
from the packet to keep its size bounded. Source: Yahoo
`/v7/finance/options/{SYMBOL}` (no API key required). Populated by running
the `OMON` command. Source: ADR-115 OMON window.

#### 2.31 Implied-vol rank / percentile (IVOL — ADR-115)

Pulled from `research::get_ivol`. Single summary line with current ATM IV,
52-week low / high, IV rank, IV percentile, and observation count; then a
**recent trail line** listing the last **8 history points** (date = IV%)
so the model can see whether IV is rising or falling into the snapshot
date. Pure compute over an in-place IV history series built from prior
OMON fetches (each compute appends today's ATM IV from the nearest-to-
money contract on the nearest expiry). Populated by running the `IVOL`
command after `OMON` has pulled the chain. Source: ADR-115 IVOL window.

#### 2.32 Seasonality (SEAG — ADR-116)

Pulled from `research::get_seasonality`. Header line lists years covered,
best month, and worst month — followed by a **monthly table** (Month / Avg /
Median / Stdev / +Years / N) and a **day-of-week table** (Day / Avg /
+Days / N). Pure compute over cached daily bars from the `HP` window, using
a BTreeMap to bucket monthly first/last close and Zeller's congruence to
derive day-of-week from dates. Populated by running the `SEAG` command.
Source: ADR-116 SEAG window.

#### 2.33 Correlation matrix (COR — ADR-116)

Pulled from `research::get_correlation`. Header line gives mean peer
correlation, highest-correlated peer, and lowest-correlated peer — followed
by a table of up to **10 peer rows**: Peer / ρ (Pearson) / β (regression
slope) / N (observations). Window is user-configurable (30–1260 trading
days); ρ computed on daily log-returns with HashMap-based date
intersection. Peers come from `research::get_peers` (ADR-109 PEERS
surface). Populated by running the `COR` command after `PEERS` + `HP`.
Source: ADR-116 COR window.

#### 2.34 Total return analysis (TRA — ADR-116)

Pulled from `research::get_total_return`. Header line gives last close,
trailing 12-month dividends, and trailing 12-month dividend yield —
followed by a **window table** (Window / Price % / Div Yield / Total % /
Annualized / N div) across standard periods (1M / 3M / 6M / YTD / 1Y /
3Y / 5Y / ITD). Each window sums price return and dividend-yield
contribution into a total return. Pure compute over cached `HP` bars
plus cached `DVD` dividend history. Populated by running the `TRA`
command. Source: ADR-116 TRA window.

#### 2.35 Technical indicators (TECH — ADR-116)

Pulled from `research::get_technicals`. Header line gives last close and
a **trend summary** derived from counting bullish vs. bearish indicator
signals — followed by an **indicator table** (Indicator / Value / Signal)
covering RSI(14) Wilder-smoothed, MACD(12,26,9) with signal / histogram,
Bollinger Bands(20,2) with %B, ATR(14), ADX(14) with +DI/−DI, and
Stochastic(14,3) %K/%D. Pure compute over cached `HP` bars. Populated
by running the `TECH` command. Source: ADR-116 TECH window.

#### 2.36 Volatility skew / smile (SKEW — ADR-116)

Pulled from `research::get_vol_skew`. Header line gives underlying price
and count of cached expiries — followed by a **nearest-expiry summary**
(expiration, DTE, ATM IV, 25Δ put/call skew) and a **strike table** of up
to **9 points** (Strike / Moneyness / Call IV / Put IV / Combined IV).
Strikes are merged call+put by integer key; ATM is the strike nearest to
the underlying; ±10% OTM put vs call IV difference drives the skew proxy.
Pure compute over cached `OMON` chain data. Populated by running the
`SKEW` command after `OMON`. Source: ADR-116 SKEW window.

#### 2.37 Sector peer comparison

Emitted only when the fundamentals row has a non-empty sector AND at least
**3 other symbols** in `self.bg.all_fundamentals` share that sector. Compares
this symbol's P/E, Forward P/E, P/B, P/S, EV/EBITDA, Profit Margin, ROE,
Beta, Short % of Float, and Dividend Yield against the sector median.

### 4. Closing question

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
| Institutional holders (legacy fundamentals source) | 5 rows | Top-5 captures >50% of float for most names |
| Recent SEC filings | 10 rows | Covers last ~2 years for an active issuer |
| Insider trades (legacy SEC source) | 5 rows | Aggregate values already cover the summary |
| Recent news | 8 articles | Matches the news window's top slice |
| Dividend history | 6 rows | Multi-year cadence visible in 6 |
| Earnings estimates | 4 periods | Forward 1Y coverage |
| Rating changes | 6 rows | Recent analyst rotation |
| Annual statements (I/B/C) | 4 periods each | 4-year trajectory |
| Management | 6 execs | Named officers typically ≤6 |
| Stock splits | 4 rows | Historical splits rarely exceed 4 |
| Insider flow — Form 4 (ADR-112 INS) | 8 rows | Last quarter-ish of filings covers sentiment |
| Institutional top holders (ADR-112 HDS) | 6 rows | Top-6 concentration coverage |
| Daily bars table (ADR-112 HP) | 10 rows | 2-week audit trail in the packet |
| EPS surprise history (ADR-112 EPS) | 8 rows | 8-quarter beat/miss record |
| World equity indices (ADR-113 WEI) | 12 rows | 12 indices cover Americas/EMEA/Asia regimes |
| Market movers (ADR-113 MOV) | 6 symbols × 3 lists | Leadership/laggards without flooding packet |
| WACC snapshot (ADR-113 WACC) | 3 lines | Derived metric — compact summary is enough |
| FX pairs per region (ADR-114 WCR) | 8 pairs × 3 regions | Majors / crosses / EM without flooding |
| Rolling beta windows (ADR-114 BETA) | typically 3 rows | 1Y / 3Y / 5Y covers both recent and structural |
| Gordon Growth DDM (ADR-114 DDM) | 2-3 lines | Single implied price + input lineage |
| Relative valuation rows (ADR-114 RV) | ≤9 metrics | Matches Fundamentals getters with peer support |
| FIGI identifiers (ADR-114 FIGI) | 3 rows | Most US names have ≤3 share classes |
| HRA rolling windows (ADR-115 HRA) | ≤10 rows | Standard 1D–ITD ladder, one row each |
| DCF projection years (ADR-115 DCF) | 3-15 rows | Matches user-tuned `projection_years` |
| SVM model rows (ADR-115 SVM) | ≤6 rows | WACC / DDM / DCF / 3 peer-multiple rows |
| OMON ATM-zone strikes (ADR-115 OMON) | 11 rows | 5 ITM + ATM + 5 OTM across both sides |
| IVOL history trail (ADR-115 IVOL) | 8 points | Trend into today without dumping full 52w |
| Seasonality months (ADR-116 SEAG) | 12 rows | One per calendar month |
| Seasonality day-of-week (ADR-116 SEAG) | 5 rows | Mon–Fri trading sessions |
| Correlation peer cells (ADR-116 COR) | 10 rows | Sector peers capped at top-10 for readability |
| Total return windows (ADR-116 TRA) | ≤8 rows | Standard 1M–ITD ladder |
| Technical indicators (ADR-116 TECH) | 6 rows | RSI / MACD / BB / ATR / ADX / Stoch |
| Volatility skew points (ADR-116 SKEW) | 9 rows per expiry | ±10% OTM window + ATM coverage |
| Daily bars required for stats | ≥20 | Needed for 20d return and ATR warm-up |

There is no global packet size limit — total size scales with the number of
symbols. A single S&P 500 symbol now produces a packet around **14-28 KB**
(up from 12-24 KB after ADR-115; ADR-116 added five per-symbol blocks —
SEAG / COR / TRA / TECH / SKEW — including SEAG's 12-row monthly table and
COR's per-peer correlation rows); a 10-symbol basket lands near **130-260
KB** (the global context is emitted only once, so multi-symbol overhead is
still bounded by the per-symbol blocks).

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
| `research::get_insider_trades` | SQLite `research_insider_trades` | ADR-112 INS window |
| `research::get_institutional_holders` | SQLite `research_institutional_holders` | ADR-112 HDS window |
| `research::get_shares_float` | SQLite `research_shares_float` | ADR-112 FLOAT window |
| `research::get_historical_price` | SQLite `research_historical_price` | ADR-112 HP window |
| `research::get_earnings_surprises` | SQLite `research_earnings_surprise` | ADR-112 EPS window |
| `research::get_wacc` | SQLite `research_wacc` | ADR-113 WACC window |
| `research::get_world_indices` | SQLite `research_world_indices` | ADR-113 WEI window |
| `research::get_market_movers` | SQLite `research_market_movers` | ADR-113 MOV window |
| `research::get_sector_performance` | SQLite `research_sector_performance` | ADR-113 INDU window |
| `research::get_currency_rates` | SQLite `research_currency_rates` | ADR-114 WCR window |
| `research::get_beta` | SQLite `research_beta` | ADR-114 BETA window |
| `research::get_ddm` | SQLite `research_ddm` | ADR-114 DDM window |
| `research::get_relative_valuation` | SQLite `research_relative_valuation` | ADR-114 RV window |
| `research::get_figi` | SQLite `research_figi` | ADR-114 FIGI window |
| `research::get_hra` | SQLite `research_hra` | ADR-115 HRA window |
| `research::get_dcf` | SQLite `research_dcf` | ADR-115 DCF window |
| `research::get_svm` | SQLite `research_svm` | ADR-115 SVM window |
| `research::get_options_chain` | SQLite `research_options_chain` | ADR-115 OMON window |
| `research::get_ivol` | SQLite `research_ivol` | ADR-115 IVOL window |
| `research::get_seasonality` | SQLite `research_seasonality` | ADR-116 SEAG window |
| `research::get_correlation` | SQLite `research_correlation` | ADR-116 COR window |
| `research::get_total_return` | SQLite `research_total_return` | ADR-116 TRA window |
| `research::get_technicals` | SQLite `research_technicals` | ADR-116 TECH window |
| `research::get_vol_skew` | SQLite `research_vol_skew` | ADR-116 SKEW window |
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
- ADR-108 / 109 / 110 / 111 / 112 / 113 / 114 / 115 / 116 — Godel parity research surfaces
