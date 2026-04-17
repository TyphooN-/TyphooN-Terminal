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
**two hundred and eighteen sub-blocks**, each of which is skipped silently when its data
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

#### 2.37 Leverage & coverage (LEV — ADR-117)

Pulled from `research::get_leverage`. Header line summarises solvency
(Debt/EBITDA, Net Debt/EBITDA, Debt/Equity, Interest Coverage, Current/Quick
ratios) and absolute magnitudes (Total Debt, Net Debt, TTM EBITDA, TTM
interest, Total Equity). A ratio table follows with **signal classification**
HEALTHY / ELEVATED / STRETCHED per row — thresholds come from the standard
solvency cones (Debt/EBITDA <2.5 healthy, <4 elevated; interest coverage
≥5 healthy, ≥2 elevated; etc.). Pure compute over cached `FA` quarterly
statements + balance sheet + Fundamentals `total_debt` / `cash_and_equivalents`
fallbacks. Populated by running the `LEV` command after `FA`. Source: ADR-117
LEV window.

#### 2.38 Earnings quality / accruals (ACRL — ADR-117)

Pulled from `research::get_accruals`. Header line gives the **trend label**
(IMPROVING / STABLE / DETERIORATING / MIXED / INSUFFICIENT) and TTM net
income, TTM free cash flow, TTM cash conversion %, and the running average.
The period table (up to 8 quarters) shows NI, FCF, cash-conv %, and a
**per-quarter quality label** (HIGH ≥90%, MEDIUM ≥60%, LOW <60%, or
NEGATIVE_NI when reported NI ≤0). Accruals = NI − FCF. Pure compute over
cached `FA` quarterly income + cashflow statements matched by date.
Populated by running the `ACRL` command after `FA`. Source: ADR-117 ACRL
window.

#### 2.39 Realized volatility cone (RVOL — ADR-117)

Pulled from `research::get_realized_vol`. Header line gives last close,
current ATM IV (from cached `IVOL`, 0 when unknown), IV/RV gap, and the
**regime label** (CHEAP_IV / FAIR_IV / RICH_IV / NO_IV_REFERENCE). A window
table follows with realized vol % (annualized) and **percentile rank** for
20d / 60d / 120d / 252d — percentile is computed against the rolling
history of the same window, so a 252d reading at 90th percentile means
current realized vol is higher than 90% of the last year's rolling 252d
observations. Needs ≥25 cached HP bars; no IV reference when `IVOL` has
not been run or returns 0. Source: ADR-117 RVOL window.

#### 2.40 FCF yield & dividend sustainability (FCFY — ADR-117)

Pulled from `research::get_fcf_yield`. Header line gives the
**sustainability label** (SAFE / STRETCHED / UNSUSTAINABLE / NO_DIVIDEND),
TTM FCF yield, dividend yield, payout-from-FCF %, payout-from-NI %, and
5-year FCF CAGR. The period table (up to 6 rows — TTM first, then up to
5 annuals) shows FCF, dividends paid, payout ratio, and period-level
yield. Labels: UNSUSTAINABLE when FCF ≤0 or payout-from-FCF >100%,
STRETCHED when >75%, SAFE otherwise. 5Y CAGR only populated when ≥5
annual rows exist. Needs cached `FA` statements and a positive market cap
from Fundamentals. Source: ADR-117 FCFY window.

#### 2.41 Short interest & days-to-cover (SHRT — ADR-117)

Pulled from `research::get_short_interest`. Header line gives
**squeeze risk** (LOW / ELEVATED / HIGH / EXTREME / INSUFFICIENT_DATA)
and the two headline numbers (short % of float, days-to-cover). A
key-value block follows with short shares, float, shares outstanding,
avg daily volume (20-day), vendor-reported short ratio, and utilization
proxy. Thresholds: short ≥30% of float OR DTC ≥10 → EXTREME; ≥20% / ≥7
→ HIGH; ≥10% / ≥4 → ELEVATED; else LOW. Needs Fundamentals
(`short_percent_of_float`, `short_ratio`, `shares_outstanding`),
cached `FLOAT` (`float_shares`), and ≥20 cached HP bars. Source:
ADR-117 SHRT window.

#### 2.42 Altman Z-Score (ALTZ — ADR-118)

Pulled from `research::get_altman_z`. Classic 5-component Z-score
Z = 1.2(WC/TA) + 1.4(RE/TA) + 3.3(EBIT/TA) + 0.6(MVE/TL) + 1.0(Sales/TA)
with **zone classification** DISTRESS (<1.81) / GRAY / SAFE (≥2.99) or
INSUFFICIENT_DATA when balance sheet / income / MVE is missing. Header
line shows the Z-score and zone; a summary line reports the five raw
inputs (working capital, retained earnings, EBIT, market value equity,
sales, total assets, total liabilities) in $M. A component table lists
each of the 5 contributions (ratio × coefficient). Prefers
`balance_annual.first()` / `income_annual.first()` with quarterly
fallback when annuals are missing. Needs cached `FA` plus a positive
market cap from Fundamentals. Source: ADR-118 ALTZ window.

#### 2.43 Piotroski F-Score (PTFS — ADR-118)

Pulled from `research::get_piotroski`. 9-point quality checklist scored
across **three categories**: Profitability (4 points — positive NI,
positive OCF, ROA↑, OCF>NI), Leverage/Liquidity (3 points — LTDebt/TA↓,
current ratio↑, no new shares), and Operating Efficiency (2 points —
gross margin↑, asset turnover↑). Header gives total F-score (0-9),
**strength label** STRONG (≥7) / MIXED / WEAK (≤3) or INSUFFICIENT_DATA,
and the current vs prior period dates. A check table follows with
PASS/FAIL and the current/prior values that drove each comparison. Needs
`FA` with ≥2 annual income statements, ≥2 annual balance sheets, and ≥1
annual cashflow statement. Source: ADR-118 PTFS window.

#### 2.44 OHLC Volatility Estimators (VOLE — ADR-118)

Pulled from `research::get_ohlc_vol`. Five volatility estimators computed
from cached HP bars over a 60-day window (user-configurable), all
annualized with √252: **Close-to-Close** (log return stdev), **Parkinson**
(range-based, 1/(4·ln2) · mean(ln(H/L)²)), **Garman-Klass** (0.5·ln(H/L)² −
(2·ln2−1)·ln(C/O)²), **Rogers-Satchell** (drift-independent:
hc·ho + lc·lo), and **Yang-Zhang** (overnight + k·oc + (1−k)·rs with
k=0.34/(1.34+(N+1)/(N−1)) — the preferred estimator). Header line gives
the preferred estimate, its label, and trading days. A table shows each
estimator with its annualized % and efficiency-vs-close ratio. Needs ≥20
valid HP bars. Source: ADR-118 VOLE window.

#### 2.45 EPS Beat Streak & Surprise (EPSB — ADR-118)

Pulled from `research::get_eps_beat`. Walks the cached
`EarningsSurprise` history (from ADR-112 ERN fetch) sorted oldest-first.
Computes beats/misses/inlines, beat rate, **current streak** (signed —
positive means active beat streak), longest beat and miss streaks,
average + median surprise %, and a **recent-4** average. Bias label:
**POSITIVE** if avg surprise > 2%, **NEGATIVE** if < -2%, else NEUTRAL.
Trend label: **ACCELERATING** if recent-4 > avg + 1%, **DECELERATING**
if recent-4 < avg - 1%, else STABLE. Header summarises bias, trend, beat
rate, and current streak; a key-value block follows with every counter
and the latest report date + surprise %. Needs ≥1 cached
`EarningsSurprise` row. Source: ADR-118 EPSB window.

#### 2.46 Price Target Dispersion (PTD — ADR-118)

Pulled from `research::get_price_target_dispersion`. Aggregates cached
`PriceTarget` (from ADR-108 UPDG fetch) against current price to compute:
**dispersion %** ((high − low) / mean × 100), **spread %** ((high − low)
/ current × 100), implied return vs median and mean targets, upside to
high, and downside to low. **Consensus label** BULLISH (implied median
≥ 10%), BEARISH (≤ -5%), else NEUTRAL; NO_COVERAGE when num_analysts ≤ 0
or no cached target. Header gives consensus / analyst count / current
price; a key-value block follows with target levels, dispersion, spread,
and all four implied-return flavours. Needs cached `UPDG` / `PT` plus a
positive current price from Fundamentals. Source: ADR-118 PTD window.

#### 2.47 Insider Activity Bias (MNGR — ADR-119)

Pulled from `research::get_insider_activity`. Windows the cached
`InsiderTrade` rows (from ADR-112 INS) over a user-tunable window
(default **90 days**) and rolls up a per-symbol insider sentiment
summary. Header line gives **bias label** (BULLISH / NEUTRAL / BEARISH)
and **conviction label** (HIGH / MEDIUM / LOW). A key-value block
follows with total trades, buy / sell / other counts, unique insiders,
gross buy $, gross sell $, net $, buy/sell ratio, net shares, and the
latest trade date. Bias BULLISH when net > +gross·0.1, BEARISH when
net < -gross·0.1, else NEUTRAL. Conviction HIGH when ≥3 unique
insiders AND |net| > $500k; MEDIUM when one condition; LOW when
neither. Needs cached `INS` rows. Source: ADR-119 MNGR window.

#### 2.48 Dividend Growth Analysis (DIVG — ADR-119)

Pulled from `research::get_divg`. Sorts cached `DividendRecord` rows
(from ADR-109 DVD) into calendar-year buckets, excludes the incomplete
current year, and computes **1Y / 3Y / 5Y CAGRs**, consecutive growth
years (run counted newest-back), and a **consistency %** (positive-
growth-year count / total-growth-year count). Header line gives the
**trend label** (GROWING / STABLE / CUTTING / NO_HISTORY), 3Y CAGR,
consecutive growth years, and consistency. An annual table follows
(most recent first) with year, total amount, payment count, and yoy
growth %. Trend GROWING when 3Y CAGR ≥ 5% AND consistency ≥ 60%,
CUTTING when 3Y CAGR < -5% OR latest annualised < prior × 0.9, else
STABLE. Source: ADR-119 DIVG window.

#### 2.49 Earnings Momentum Trend (EARM — ADR-119)

Pulled from `research::get_earm`. Fuses cached `FA.income_quarterly`
(from ADR-110 FA) with cached `EarningsSurprise` history (from
ADR-112 ERN) to compute a **0-100 composite momentum score**.
Compares the most-recent-4-quarter revenue yoy growth against the
prior-4-quarter yoy growth, layers the EPS surprise acceleration on
top, and labels **ACCELERATING** (score ≥ 65), **DECELERATING**
(score ≤ 35), or **STABLE**. Header line shows the composite score,
momentum label, and quarters used. A key-value block reports recent
revenue growth %, prior revenue growth %, revenue acceleration %,
recent EPS surprise %, prior EPS surprise %, and EPS surprise
acceleration %. A quarter table shows up to **8 quarters** with
period / revenue / revenue yoy % / EPS actual / EPS estimate / EPS
surprise %. Needs ≥5 quarters of cached income statements and
cached surprise history. Source: ADR-119 EARM window.

#### 2.50 Sector Rotation Strength (SECTR — ADR-119)

Pulled from `research::get_sector_rotation`. Ranks the symbol's
Fundamentals sector among cached `SectorPerformance` rows (from
ADR-113 INDU) and derives a relative strength label. Header line
gives **strength label** (LEADER / NEUTRAL / LAGGARD / NO_DATA),
the symbol's sector, its sector rank (e.g. 2/11), and the sector's
change %. A key-value block follows with symbol sector change %,
sectors total, avg sector change %, median sector change %,
**relative strength %** (symbol sector change − avg), breadth %,
strongest sector + change %, and weakest sector + change %. LEADER
when the symbol's sector ranks in the top third AND relative
strength > 0. LAGGARD when in the bottom third AND relative
strength < 0. NEUTRAL otherwise. Source: ADR-119 SECTR window.

#### 2.51 Upgrade/Downgrade Momentum (UPDM — ADR-119)

Pulled from `research::get_updm`. Buckets cached `RatingChange` rows
(from ADR-109 UPDG) into **30d / 90d / 180d** windows and classifies
each action via case-insensitive substring match: "upgrad",
"downgrad", "initiat", "maintain". Header line gives **bias label**
(BULLISH / NEUTRAL / BEARISH) and **trend label** (IMPROVING /
STABLE / DETERIORATING). A key-value block follows with net 30d /
90d / 180d, upgrades / downgrades at each window, initiations 90d,
maintains 90d, and the latest action (date, firm, to-grade). Bias
BULLISH when net_90d ≥ 2, BEARISH when ≤ -2, else NEUTRAL. Trend
IMPROVING when net_30d > net_90d / 3, DETERIORATING when net_30d <
-net_90d / 3, else STABLE. Source: ADR-119 UPDM window.

#### 2.52 Momentum 12-1 (MOM — ADR-120)

Pulled from `research::get_momentum`. The Jegadeesh-Titman
12-month-minus-1-month momentum score over cached `HP` bars, requiring
**≥252 bars**. Header line gives the **regime label** (STRONG /
NEUTRAL / WEAK / CRASH / INSUFFICIENT_DATA), **trend label**
(ACCELERATING / STABLE / DECELERATING), composite score (0-100), and
bars used. A key-value block follows with 1m / 3m / 6m / 12m / 12-1
returns %, annualised vol %, and the vol-adjusted score. Composite =
`50 + vol_adj·20 + 6m·0.3`, clamped [0, 100]. Regime STRONG ≥75,
NEUTRAL ≥40, WEAK ≥20, else CRASH. Trend ACCELERATING when 1m > 3m/3
AND 3m > 6m/2; DECELERATING when both are reversed; STABLE otherwise.
Source: ADR-120 MOM window.

#### 2.53 Liquidity Profile (LIQ — ADR-120)

Pulled from `research::get_liquidity`. Rolls up cached `HP` bars and
`Fundamentals.shares_outstanding` over a user-tunable window
(default **60 days**, min 20). Header line gives the **tier label**
(DEEP / LIQUID / MODERATE / THIN / ILLIQUID / INSUFFICIENT_DATA), avg
$ / day, and median $ / day. A key-value block follows with avg
shares / day, daily turnover % (against shares outstanding), Amihud
illiquidity ×1e6, ATR %, and Corwin-Schultz spread proxy %. Tier
thresholds on avg daily dollar volume: DEEP ≥$500M, LIQUID ≥$50M,
MODERATE ≥$5M, THIN ≥$500K, ILLIQUID below. Source: ADR-120 LIQ
window.

#### 2.54 Breakout Proximity (BREAK — ADR-120)

Pulled from `research::get_breakout`. Tracks the symbol's position
inside its 20d / 60d / 52w ranges over cached `HP` bars, requiring
**≥20 bars**. Header line gives the **breakout label** (NEW_HIGH /
NEAR_HIGH / MID_RANGE / NEAR_LOW / NEW_LOW), **setup label**
(BREAKOUT_IMMINENT / CONSOLIDATING / TRENDING_UP / TRENDING_DOWN /
NEUTRAL), and last close. A key-value block follows with 20d / 60d /
52w highs and lows, distance from 52w high / low (%), position in
52w range (%), and consolidation % (20d range / mean close). Setup
BREAKOUT_IMMINENT when consolidation < 8% AND position in 20d range
≥ 70%; CONSOLIDATING when < 6%; TRENDING_UP / DOWN when near the
60d high / 52w low with matching range position; NEUTRAL otherwise.
Source: ADR-120 BREAK window.

#### 2.55 Cash Conversion Cycle (CCRL — ADR-120)

Pulled from `research::get_cash_cycle`. Computes DSO + DIO - DPO
over cached `FA` statements (annual preferred, days factor 365;
quarterly fallback, days factor 91.25). Header line gives the
**efficiency label** (EFFICIENT / NEUTRAL / INEFFICIENT /
INSUFFICIENT_DATA), **trend label** (IMPROVING / STABLE /
DETERIORATING), latest CCC days, prior CCC days, change days, and
3y avg CCC days. A key-value block reports the latest DSO / DIO /
DPO components. A per-period table follows (up to **8 rows**) with
period / DSO / DIO / DPO / CCC. Efficiency EFFICIENT <30 days,
NEUTRAL <90, else INEFFICIENT. Trend IMPROVING when change ≤ -5,
DETERIORATING when ≥ +5, STABLE otherwise. Source: ADR-120 CCRL
window.

#### 2.56 Unified Credit Score (CREDIT — ADR-120)

Pulled from `research::get_credit`. Fuses the cached ALTZ / PTFS /
LEV / ACRL snapshots from Rounds 10 / 11 into a single 0-100
weighted score (weights **35 / 25 / 25 / 15**). Header line gives
the **letter grade** (AAA ≥90 / AA ≥80 / A ≥70 / BBB ≥60 / BB ≥50 /
B ≥35 / CCC / INSUFFICIENT_DATA), **credit label**
(INVESTMENT_GRADE / BORDERLINE / SPECULATIVE / DISTRESSED), composite
score, and inputs available (0..4). A key-value block follows with
Altman Z + zone, Piotroski F + label, leverage summary label,
accruals trend label, and TTM cash conversion %. A component table
(up to **6 rows**) reports each populated component: name / value /
score / weight % / contribution. Returns INSUFFICIENT_DATA when
none of ALTZ / PTFS / LEV / ACRL is cached. Source: ADR-120 CREDIT
window.

#### 2.57 GARP Composite (GROWM — ADR-121)

Pulled from `research::get_growm`. Meta-composite that fuses the
Round 13 MOM composite, the Round 12 EARM composite, and the Round
12 DIVG snapshot into a single 0-100 score with weights **40 / 40 /
20**. Header line gives the **garp_label** (GARP / GROWTH / VALUE /
SPECULATIVE / NO_DATA), composite score, and inputs_available
(0..3). A key-value block reports momentum regime + score, earnings
trend label + score, and dividend 3y CAGR + trend. A component
table (up to **5 rows**) lists each populated component: name /
value / score / weight % / contribution. Returns NO_DATA if none
of MOM / EARM / DIVG are cached. Source: ADR-121 GROWM window.

#### 2.58 Smart-Money Flow (FLOW — ADR-121)

Pulled from `research::get_flow`. Windowed net flow from cached
`InsiderTrade` + `InstitutionalHolder` rows (default **90-day**
window, user-tunable 7..365). Header line gives the **flow_label**
(STRONG_BUY / BUY / NEUTRAL / SELL / STRONG_SELL / NO_DATA),
composite score, insider sub-score, institutional sub-score, and
window in days. Body block reports insider buy / sell / net USD
volumes, trade count + unique insiders, institutional buyers /
sellers / holders tracked + net ratio + share delta. Composite
weights **insider 60 / institutional 40** when both sides present;
single-side when only one. Source: ADR-121 FLOW window.

#### 2.59 Market Regime (REGIME — ADR-121)

Pulled from `research::get_regime`. Meta-composite that fuses the
Round 8 VOLE realized-vol snapshot, the Round 7 TECH technicals
ADX field, and the Round 7 HRA 1Y return + Sharpe into a single
regime label (TRENDING / MEAN_REVERTING / VOLATILE / QUIET /
INSUFFICIENT_DATA). Header line gives label, composite score,
and inputs_available (0..3). Body block reports realized vol %
(with source), ADX value + trend summary, 1Y return, Sharpe,
trend-strength sub-score, volatility sub-score (inverse — lower
vol = higher score), return sub-score. Label logic: VOLATILE
first if vol ≥ 40 %, TRENDING if ADX ≥ 25 and return positive,
QUIET if vol < 20 % and ADX < 18, else MEAN_REVERTING.
Source: ADR-121 REGIME window.

#### 2.60 Relative Volume (RELVOL — ADR-121)

Pulled from `research::get_relvol`. Pure compute over cached HP
bars (≥20 bars). Header line gives the **activity_label** (EXTREME
≥3× / HIGH ≥2× / ELEVATED ≥1.5× / NORMAL / LOW <0.5× /
INSUFFICIENT_DATA), **direction_label** (BULLISH / BEARISH /
NEUTRAL from current close vs prior ± 0.5 %), and the 20-day
relative volume. Body block reports current volume, 5d / 20d /
60d trailing averages, rel-vol ratios 5d / 20d / 60d, 5d-vs-20d
volume trend %, 60-day percentile rank of the current bar's
volume, and bars used. Averages deliberately exclude the current
bar to prevent self-skew. Source: ADR-121 RELVOL window.

#### 2.61 Margin Trajectory (MARGINS — ADR-121)

Pulled from `research::get_margins`. Pure compute over cached FA
statements (annual preferred, quarterly fallback). Header line
gives the **overall_trend_label** (EXPANDING / STABLE /
CONTRACTING — majority across gross / op / net), **quality_label**
(HIGH ≥20 % / MEDIUM ≥8 % / LOW — latest op margin bucket),
basis, and latest period. A 4-column table (metric / latest /
prior / change+trend) shows gross, operating, and net margin
rows. An averages block reports across-period means. A per-period
history table follows (up to **6 rows**) with gross / op / net %
for each period. Returns INSUFFICIENT_DATA when the FA statements
cache is empty. Source: ADR-121 MARGINS window.

#### 2.62 Value-Factor Composite (VAL — ADR-122)

Pulled from `research::get_val`. Meta-composite that fuses six
valuation ratios (P/E, Forward P/E, P/B, P/S, EV/EBITDA, FCF Yield)
against sector-peer medians into a single 0-100 score with weights
**25 / 15 / 15 / 15 / 20 / 10**. Header line gives the **value_label**
(DEEP_VALUE / VALUE / FAIR / EXPENSIVE / PREMIUM / NO_DATA),
composite score, inputs_available (0..6), and peers_considered +
sector. Body block reports each metric and its sector median
pairwise. A component table (up to **6 rows**) lists each populated
component: name / value / score / weight % / contribution. Lower-
better scoring (ratio ≤0.5× median → 100, ≥2.0× → 0); FCFY uses
higher-better. Returns NO_DATA when no inputs usable. Source:
ADR-122 VAL window.

#### 2.63 Quality-Factor Composite (QUAL — ADR-122)

Pulled from `research::get_qual`. Meta-composite that fuses Round 10
PTFS (Piotroski F-score), Round 14 MARGINS (operating margin trend),
Round 10 ACRL (cash conversion / accruals trend), and Round 10 LEV
(leverage summary) into a single 0-100 score with weights **30 / 25 /
25 / 20**. Header line gives the **quality_label** (HIGH_QUALITY /
QUALITY / AVERAGE / POOR / WEAK / NO_DATA), composite score, and
inputs_available (0..4). Body block reports Piotroski F + label,
operating margin + trend, cash conversion + accruals trend, leverage
summary + debt/EBITDA. A component table (up to **4 rows**) lists
each populated component. Source: ADR-122 QUAL window.

#### 2.64 Risk-Factor Composite (RISK — ADR-122)

Pulled from `research::get_risk`. Meta-composite that fuses Round 8
VOLE (realized vol), BETA (beta_1y), Round 13 LIQ (liquidity tier),
Round 10 SHRT (short % float + DTC), and Round 10 ALTZ (Altman Z)
into a single 0-100 score with weights **25 / 20 / 15 / 15 / 25**.
**Higher composite = RISKIER.** Header line gives the **risk_label**
(LOW_RISK / MODERATE / ELEVATED / HIGH_RISK / DISTRESSED / NO_DATA),
composite score, and inputs_available (0..5). DISTRESSED is a
single-factor veto from Altman Z zone — it overrides numeric
thresholds. Body block reports realized vol, beta_1y, liquidity tier,
short % float + days to cover, Altman Z + zone. A component table
(up to **5 rows**) lists each populated component. Source: ADR-122
RISK window.

#### 2.65 Insider Streak Detector (INSSTRK — ADR-122)

Pulled from `research::get_insstrk`. Pure post-processing over cached
`InsiderTrade` rows (user-tunable window, default **180 days**).
Groups trades by insider name, finds each insider's longest
consecutive-direction run, tallies buy-streak / sell-streak counts,
and emits a single **streak_label** (STRONG_ACCUMULATION /
ACCUMULATION / DISTRIBUTION / STRONG_DISTRIBUTION / MIXED / NONE).
Header line gives label, unique_insiders, window_days, and streak
counts. Body block reports buy_streak_count, sell_streak_count,
longest_buy_streak, longest_sell_streak, net buy / sell USD totals.
Per-insider streak table (up to **8 rows**) lists name / direction /
consecutive events / net $ / net shares / first + latest date.
STRONG_ACCUMULATION fires when buy_streak_count ≥ 3 **and**
longest_buy_streak ≥ 4; symmetric for distribution. BTreeMap-based
grouping gives deterministic row ordering for LAN-sync byte parity.
Source: ADR-122 INSSTRK window.

#### 2.66 Analyst Coverage (COVG — ADR-122)

Pulled from `research::get_covg`. Fuses Round 7 PriceTarget (coverage
size), AnalystRecommendations (5-bucket consensus distribution), and
Round 12 UPDM (90d upgrades / downgrades) into three sub-scores plus
a composite. Header line gives the **coverage_label** (EXPANDING /
STABLE / CONTRACTING / THIN / NONE), composite score, num_analysts,
and inputs_available (0..3). Composite weights: breadth 35 /
consensus 35 / churn 30. Body block reports num_analysts, target
mean / low / high, 5-bucket consensus counts + total + bull_ratio,
90d upgrades / downgrades / net / churn, and the three sub-scores.
Label logic: THIN if num_analysts < 5; EXPANDING if net_90d ≥ 3 AND
breadth ≥ 70; CONTRACTING if net_90d ≤ -3; STABLE otherwise.
Churn score is centred at 50 (neutral) so "no activity" doesn't bias
the composite down. Source: ADR-122 COVG window.

#### 2.67 Value Rank (VRK — ADR-123)

Pulled from `research::get_vrk`. Percentile rank of the Round 15 VAL
composite_score within the symbol's sector cohort. Header line gives
**rank_label** (TOP_DECILE / TOP_QUARTILE / ABOVE_MEDIAN / BELOW_MEDIAN
/ BOTTOM_QUARTILE / BOTTOM_DECILE / NO_DATA), percentile_rank,
rank_position (1-based from best value), cohort size (peers_considered
+ 1), sector, and as_of. Body block reports the subject composite,
sector median / p25 / p75 VAL scores, and peers_considered / with_data.
Requires ≥3 peers in the same sector with VAL snapshots. Label ladder:
TOP_DECILE ≥90, TOP_QUARTILE ≥75, ABOVE_MEDIAN ≥50, BELOW_MEDIAN ≥25,
BOTTOM_QUARTILE ≥10, BOTTOM_DECILE <10. Source: ADR-123 VRK window.

#### 2.68 Quality Rank (QRK — ADR-123)

Pulled from `research::get_qrk`. Percentile rank of the Round 15 QUAL
composite_score within the symbol's sector cohort. Same shape as VRK
with the same label ladder. Because `QualitySnapshot` does not carry
sector directly, the broker handler cross-joins with
`fundamentals::get_fundamentals` per peer to filter the cohort. Body
block reports subject composite, sector median / p25 / p75 QUAL
scores, and peers_considered / with_data. Source: ADR-123 QRK window.

#### 2.69 Risk Rank (RRK — ADR-123)

Pulled from `research::get_rrk`. Percentile rank of the Round 15 RISK
composite_score within the symbol's sector cohort. **Critical
inversion:** RISK composite is higher = riskier, so this snapshot
inverts the percentile such that higher `percentile_rank` here =
SAFER than peers, and the label ladder reads SAFEST_DECILE /
SAFEST_QUARTILE / ABOVE_MEDIAN_SAFE / BELOW_MEDIAN_RISKY /
BOTTOM_QUARTILE_RISKY / RISKIEST_DECILE / NO_DATA. The header line
shows "higher pct = SAFER" to make the inversion explicit. Same
sector cross-join pattern as QRK. Source: ADR-123 RRK window.

#### 2.70 Relative EPS Growth (RELEPSGR — ADR-123)

Pulled from `research::get_relepsgr`. Computes 3-year EPS CAGR from
`FinancialStatements.income_annual[].eps` (requires ≥4 annual rows)
and compares to the sector median CAGR. Header line gives the
**relative_label** (FAR_ABOVE / ABOVE / INLINE / BELOW / FAR_BELOW /
CAGR_NEGATIVE / NO_DATA), symbol CAGR %, gap to median in percentage
points, sector, and as_of. Body block reports latest / earliest EPS
(with years_used), sector median / p25 / p75 CAGR %, and
peers_considered / with_data. Labels are keyed on `gap_to_median_pp`
(FAR_ABOVE ≥ +10pp, ABOVE ≥ +3, INLINE within ±3, BELOW ≤ -3,
FAR_BELOW ≤ -10). CAGR_NEGATIVE overrides when the subject's EPS
series has a non-positive endpoint, in which case a linear growth
proxy is used for `symbol_cagr_pct`. Source: ADR-123 RELEPSGR window.

#### 2.71 Post-Earnings Drift (PEAD — ADR-123)

Pulled from `research::get_pead`. Joins cached `EarningsSurprise`
rows with cached `HistoricalPriceRow` bars to measure average
forward drift over 1 / 3 / 5 / 10 trading days after each
announcement. Header line gives the **drift_direction_label**
(DRIFT_UP / DRIFT_DOWN / MIXED / INSUFFICIENT_DATA), events_used,
and avg 5d drift %. Body block reports avg drift at all four
horizons, BEAT-event 5d drift, MISS-event 5d drift, the latest
event's date / surprise% / 5d drift, and events_used / in_cache.
Requires ≥3 cached surprises and ≥11 HP bars per event (t0 + 10
forward) for inclusion. BEAT / MISS / INLINE classification uses a
±1 % surprise threshold. Source: ADR-123 PEAD window.

#### 2.72 Size Factor (SIZEF — ADR-124)

Pulled from `research::get_sizef`. Computes a sector-relative
percentile rank of `log(market_cap)` plus an absolute tier label
(MEGA_CAP ≥ $200B, LARGE_CAP ≥ $10B, MID_CAP ≥ $2B, SMALL_CAP ≥
$300M, MICRO_CAP > $0). Header line gives tier + rank label +
rank position within sector cohort + percentile. Body block
reports subject market cap, log(cap), sector median / p25 / p75
caps in $B, and peers considered / with data. Requires ≥3
sector peers with positive market_cap in `get_all_fundamentals`.
Source: ADR-124 SIZEF window.

#### 2.73 Momentum Rank (MOMF — ADR-124)

Pulled from `research::get_momf`. Sector-relative percentile rank
of Round 10 `MomentumSnapshot.composite_score` within the same
sector. Header line gives the **rank_label** (TOP_DECILE /
TOP_QUARTILE / ABOVE_MEDIAN / BELOW_MEDIAN / BOTTOM_QUARTILE /
BOTTOM_DECILE / NO_DATA), rank_position within the cohort, and
raw percentile. Body block reports the subject composite, the
sector median / p25 / p75, and peers_considered / peers_with_data.
Peers with `regime_label == INSUFFICIENT_DATA` are dropped before
ranking. Requires ≥3 sector peers with MOMENTUM cached. Source:
ADR-124 MOMF window.

#### 2.74 PEAD Rank (PEADRANK — ADR-124)

Pulled from `research::get_peadrank`. Sector-relative percentile
rank of `PeadSnapshot.avg_drift_5d_pct` within the same sector —
answers "how does this name's post-earnings drift compare to its
peers?" Header line gives the rank label, rank_position, and raw
percentile. Body block reports the subject's avg 5d drift, the
sector median / p25 / p75 5d drift percentages, and
peers_considered / peers_with_data. Both subject and peers are
filtered by `drift_direction_label != INSUFFICIENT_DATA &&
events_used >= 3`. Source: ADR-124 PEADRANK window.

#### 2.75 Fundamental Quality Meter (FQM — ADR-124)

Pulled from `research::get_fqm`. A fused Piotroski + margins +
accruals composite that deliberately excludes leverage (the
differentiator from Round 15 QUAL). Weights: PTFS 40 / MARGINS 30
/ ACRL 30. Header line gives the **operator_label** (ELITE_OPERATOR
≥85 / STRONG_OPERATOR ≥70 / AVERAGE_OPERATOR ≥50 / WEAK_OPERATOR
≥30 / BROKEN_OPERATOR <30 / NO_DATA) and composite score. Body
block reports the Piotroski score + label, operating margin % +
trend, cash conversion % + trend, and inputs_available (1-3).
Emits a row whenever at least one of PTFS / MARGINS / ACRL is
cached. Source: ADR-124 FQM window.

#### 2.76 Relative Revenue Growth (REVRANK — ADR-124)

Pulled from `research::get_revrank`. Computes the 3-year compound
annual growth rate from `FinancialStatements.income_annual[].revenue`
(requires ≥4 annual rows), compares to the sector median CAGR, and
emits a gap-to-median in percentage points. Header line gives the
**relative_label** (FAR_ABOVE / ABOVE / INLINE / BELOW / FAR_BELOW
/ CAGR_NEGATIVE / INSUFFICIENT_DATA), gap_to_median_pp, and
symbol_cagr_pct. Body block reports latest / earliest revenue in
$B with years_used, the sector median / p25 / p75 CAGRs, and
peers_considered / peers_with_data. CAGR_NEGATIVE is emitted when
the revenue series has a non-positive endpoint (rare but handled
for symmetry with RELEPSGR). Source: ADR-124 REVRANK window.

#### 2.77 Leverage Rank (LEVRANK — ADR-125)

Pulled from `research::get_levrank`. Sector-relative percentile
rank of **debt-to-equity** from the Round 15 LEV cache, computed
with `higher_is_better=false` so a *lower* D/E earns a *higher*
(safer) percentile. Header line gives the risk-inverted
**rank_label** (SAFEST_DECILE / SAFE / ... / RISKIEST_DECILE /
NEGATIVE_EQUITY / NO_DATA / INSUFFICIENT_DATA). Body block reports
subject D/E, rank position within sector cohort, sector median /
p25 / p75 D/E, and peers_considered / peers_with_data. The
NEGATIVE_EQUITY branch fires when `total_equity <= 0` and replaces
the rank line with raw total_debt / total_equity levels in $B.
Source: ADR-125 LEVRANK window.

#### 2.78 Operating Quality Rank (OPERANK — ADR-125)

Pulled from `research::get_operank`. Sector-relative percentile
rank of **`MarginsSnapshot.latest_operating_margin_pct`** from the
Round 14 MARGINS cache. Isolates the pricing-power signal from
the fused FQM/QUAL composites. Header line gives the standard
**rank_label** (TOP_DECILE...BOTTOM_DECILE / NO_DATA /
INSUFFICIENT_DATA), operating margin %, and margin trend label.
Body block reports subject op margin with trend, rank position,
sector median / p25 / p75 op margin %, and peers_considered /
peers_with_data. Filters peers whose `periods_used > 0`. Source:
ADR-125 OPERANK window.

#### 2.79 FQM Rank (FQMRANK — ADR-125)

Pulled from `research::get_fqmrank`. Sector-relative percentile
rank of **`FundamentalQualityMeterSnapshot.composite_score`** from
the Round 17 FQM cache. The natural rank overlay for the
deliberately-leverage-free operator-quality composite added in
Round 17. Header line gives `operator_label / rank_label` and the
composite score. Body block reports subject composite, rank
position, sector median / p25 / p75 composite, and
peers_considered / peers_with_data. Both subject and peers are
filtered by `operator_label != "NO_DATA" && composite_score > 0`.
Source: ADR-125 FQMRANK window.

#### 2.80 Liquidity Rank (LIQRANK — ADR-125)

Pulled from `research::get_liqrank`. Sector-relative percentile
rank of **`LiquiditySnapshot.avg_daily_dollar_volume`** from the
Round 13 LIQ cache. Higher ADV$ = deeper = higher rank. Header
line gives `tier_label / rank_label` so the reader can
distinguish "deep for this sector" from "deep absolutely." Body
block reports subject ADV$ in $M, rank position, sector median /
p25 / p75 ADV$ in $M, and peers_considered / peers_with_data.
Filters peers whose `liquidity_tier != "INSUFFICIENT_DATA"`.
Source: ADR-125 LIQRANK window.

#### 2.81 Earnings Surprise Streak (SURPSTK — ADR-125)

Pulled from `research::get_surpstk`. Pure symbol-local
time-series stat over the cached `EarningsSurprise` rows — no
sector needed, no peer cross-join. Classifies each historical
event via a ±2% band around `surprise_pct` (BEAT > +2%, MISS <
-2%, INLINE in between), counts consecutive and longest streaks,
and maps to a streak ladder. Header line gives the
**streak_label** (HOT_STREAK / BEAT_TREND / MIXED / MISS_TREND /
COLD_STREAK / INSUFFICIENT_DATA). Body block reports total events,
beats / misses / inlines breakdown, beat rate %, current streak
length + type, longest beat and miss streaks, avg surprise %, and
latest event date + label + surprise %. Requires ≥4 events to
move out of INSUFFICIENT_DATA. Source: ADR-125 SURPSTK window.

#### 2.82 Dividend Growth Rank (DVDRANK — ADR-126)

Pulled from `research::get_dvdrank`. Sector-relative percentile
rank of **`DivgSnapshot.cagr_3y_pct`** from the Round 12 DIVG
cache. Higher CAGR = higher rank. Peers whose
`trend_label = "NO_HISTORY"` are filtered so the cohort captures
only names with enough history to compute a meaningful CAGR.
Header line gives the standard **rank_label** (TOP_DECILE ... →
BOTTOM_DECILE / NO_DATA / INSUFFICIENT_DATA), the subject's 3y
CAGR, consecutive_growth_years, and DIVG trend_label. Body block
reports subject 3y CAGR, rank position within sector cohort,
sector median / p25 / p75 CAGR, and peers_considered /
peers_with_data. Source: ADR-126 DVDRANK window.

#### 2.83 Earnings Momentum Rank (EARMRANK — ADR-126)

Pulled from `research::get_earmrank`. Sector-relative percentile
rank of **`EarmSnapshot.composite_score`** from the Round 12 EARM
cache. The natural rank overlay for the earnings-momentum composite
that fuses EPS surprise history + revision trend + growth slope.
Header line gives the standard **rank_label** and the subject's
`momentum_label` (ACCELERATING / STABLE / DECELERATING /
INSUFFICIENT_DATA). Body block reports subject composite, rank
position, sector median / p25 / p75 composite, and peers_considered
/ peers_with_data. Filters peers whose
`momentum_label != "INSUFFICIENT_DATA"`. Source: ADR-126 EARMRANK
window.

#### 2.84 Upgrade/Downgrade Rank (UPDGRANK — ADR-126)

Pulled from `research::get_updgrank`. Sector-relative percentile
rank of **`UpdmSnapshot.net_90d`** from the Round 12 UPDM cache —
the net sell-side upgrade-minus-downgrade count over the trailing
90 days. Higher net = more analyst conviction = higher rank. Header
line gives the standard **rank_label** and the subject's
`bias_label` (BULLISH / NEUTRAL / BEARISH / NO_COVERAGE). Body
block reports subject net 90d, rank position, sector median / p25
/ p75 net, and peers_considered / peers_with_data. Filters peers
whose `bias_label != "NO_COVERAGE"` so the cohort captures
sell-side-active names only. Source: ADR-126 UPDGRANK window.

#### 2.85 Gap Yearly (GY — ADR-126)

Pulled from `research::get_gy`. Pure symbol-local time-series stat
over the most recent **253 bars** of the HP cache — no sector, no
peer cross-join. For each adjacent pair, computes the overnight
gap as `(open − prev_close) / prev_close × 100`, skips gaps below
the 0.01% noise floor, and bins the remaining gaps at 2% / 5% /
10% thresholds in both directions. Header line gives the
**gap_label** (EXPLOSIVE / GAPPY / NORMAL / SMOOTH /
INSUFFICIENT_DATA), bars used, and total gaps. Body block reports
the 2/5/10% bin counts in each direction, largest up gap + date,
largest down gap + date, and average absolute gap %. Useful as an
event-driven risk measure. Requires ≥20 HP bars to emit. Source:
ADR-126 GY window.

#### 2.86 Daily Event Streak (DES — ADR-126)

Pulled from `research::get_des`. Pure symbol-local time-series stat
over the same 253-bar HP window as GY. Classifies each
close-over-close move as UP / DOWN / FLAT, computes the longest
up-streak and down-streak over the window, the current trailing
streak, the up-day rate (excluding flat days from the denominator),
and the average up-day and down-day move %. Header line gives the
**streak_label** (STRONG_UPTREND / UPTREND_BIAS / NEUTRAL /
DOWNTREND_BIAS / STRONG_DOWNTREND / INSUFFICIENT_DATA), the up-day
rate %, and the current streak `type × length`. Body block reports
bars used, up / down / flat day counts, longest up and down
streaks, and avg up / down move %. Complements SURPSTK (earnings
streak) with a price-action streak. Requires ≥20 HP bars to emit.
Source: ADR-126 DES window.

#### 2.87 Dividend Yield Rank (DVDYIELDRANK — ADR-127)

Pulled from `research::get_dvdyieldrank`. Sector-relative percentile
rank of current dividend yield (`Fundamentals.dividend_yield`). Non-
payers (None or 0.0) are filtered out on both subject and peer sides
so the cohort captures dividend-paying names only. Header line gives
the **rank_label** (TOP_DECILE / TOP_QUARTILE / ABOVE_MEDIAN /
BELOW_MEDIAN / BOTTOM_QUARTILE / BOTTOM_DECILE / INSUFFICIENT_DATA /
NO_DATA), subject yield, rank position, and sector. Body block reports
sector median / p25 / p75 yields, peers considered, and peers with
data. Companion to DVDRANK (which ranks dividend *growth*, not *yield*).
Requires ≥3 sector peers paying dividends. Source: ADR-127
DVDYIELDRANK window.

#### 2.88 Short Interest Rank (SHRANK — ADR-127)

Pulled from `research::get_shrank`. Sector-relative percentile rank of
`Fundamentals.short_percent_of_float`, **risk-inverted** so a lower
short interest earns a higher (safer) rank. Header line gives the
**rank_label** (SAFEST_DECILE / SAFEST_QUARTILE / ABOVE_MEDIAN_SAFE /
BELOW_MEDIAN_RISKY / BOTTOM_QUARTILE_RISKY / RISKIEST_DECILE /
INSUFFICIENT_DATA / NO_DATA), subject short %, rank position, and
sector. Body block reports sector median / p25 / p75 short % and
peer counts. Replaces the originally-planned INSIDERCONC (which
required a new Fundamentals field that doesn't exist in the cache).
Requires ≥3 sector peers with short interest data. Source: ADR-127
SHRANK window.

#### 2.89 Annualized ATR (ATRANN — ADR-127)

Pulled from `research::get_atrann`. Pure symbol-local time-series
stat. Computes the 14-period Wilder Average True Range over the most
recent 253 sessions of cached HP bars, expresses it as a percent of
the latest close, and annualizes via √252. Header line gives the
**regime_label** (LOW_VOL < 15% < NORMAL_VOL < 30% < HIGH_VOL < 60% <
EXTREME_VOL / INSUFFICIENT_DATA), ATR14 absolute and %, annualized %,
and bars used. Body block reports latest close, ATR14 in price units,
ATR14 %, and annualized %. Complements IVOL (implied vol, ADR-115)
with a realized-vol regime surface that works without options data.
Requires ≥15 HP bars. Source: ADR-127 ATRANN window.

#### 2.90 Drawdown History (DDHIST — ADR-127)

Pulled from `research::get_ddhist`. Pure symbol-local time-series stat
over the same 253-bar HP window. Tracks the deepest peak-to-trough
drawdown with peak/trough dates, the longest drawdown duration in
sessions, the count of 5% and 10% corrections (local-peak-to-trough
declines), and the current drawdown from the running peak. Header
line gives the **regime_label** (RECOVERING > -1% / SHALLOW > -10% /
MEANINGFUL > -20% / SEVERE > -35% / CATASTROPHIC / INSUFFICIENT_DATA),
max dd %, current dd %, and bars used. Body block reports max dd
peak / trough dates, longest drawdown in sessions, corrections ≥5% /
≥10%, and current dd %. Requires ≥20 HP bars. Source: ADR-127 DDHIST
window.

#### 2.91 Price Performance (PRICEPERF — ADR-127)

Pulled from `research::get_priceperf`. Pure symbol-local time-series
stat. Computes total returns at 1M (21 sessions), 3M (63), 6M (126),
YTD (from the first bar of as_of's calendar year), and 1Y (253)
lookbacks over cached HP bars. Header line gives the **trend_label**
(STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR /
INSUFFICIENT_DATA), latest close, 1Y return, YTD return, and bars
used. Body block reports 1M / 3M / 6M returns, YTD / 1Y returns, bars
used, and latest close. Trend label gates on `bars_used >= 20` so
shallow histories emit INSUFFICIENT_DATA rather than noise. Source:
ADR-127 PRICEPERF window.

#### 2.92 Beta Rank vs Sector (BETARANK — ADR-128)

Pulled from `research::get_betarank`. Direct companion to SHRANK.
Risk-inverted percentile rank of `Fundamentals.beta` within the
same sector — *lower* beta earns a *higher* (safer) rank. Header
line gives the **rank_label** (SAFEST_DECILE / SAFEST_QUARTILE /
ABOVE_MEDIAN_SAFE / BELOW_MEDIAN_RISKY / BOTTOM_QUARTILE_RISKY /
RISKIEST_DECILE / INSUFFICIENT_DATA), subject beta, sector
median/p25/p75, and percentile. Body block reports subject beta,
sector median, p25, p75, safer-than count, and percentile. Needs
≥3 sector peers with a non-None beta. Source: ADR-128 BETARANK
window. Negative-beta subjects correctly land at SAFEST_DECILE
against conventional positive-beta peers.

#### 2.93 PEG Ratio Rank vs Sector (PEGRANK — ADR-128)

Pulled from `research::get_pegrank`. Value-inverted percentile
rank of `Fundamentals.peg_ratio` — *lower* PEG (cheaper growth)
earns a *higher* (better-value) rank. Fills the gap where VAL
(Round 15) fuses P/E, Forward P/E, P/B, P/S, EV/EBITDA, FCF yield
but not PEG. Header line gives the **rank_label** (TOP_DECILE …
BOTTOM_DECILE), subject PEG, sector median/p25/p75, and
percentile. Body block reports subject PEG, sector median, p25,
p75, beat count, and percentile. Non-positive / non-finite PEG is
filtered on both subject and peer sides. Source: ADR-128 PEGRANK
window.

#### 2.94 52-Week High/Low Distance (FHIGHLOW — ADR-128)

Pulled from `research::get_fhighlow`. Pure symbol-local HP stat
over the trailing 253-session window. Tracks max/min close,
high/low dates, days since each, percent-from-high,
percent-from-low, and range position (0 = at low, 100 = at high).
Header line gives the **proximity_label** (AT_HIGH / NEAR_HIGH /
MID_RANGE / NEAR_LOW / AT_LOW / INSUFFICIENT_DATA), latest close,
pct-from-high, pct-from-low, and range position. Body block
reports 52w high + date + days-since, 52w low + date + days-since,
pct-from-high/low, range position, and bars used. Falls back to
mid-range (50.0) on degenerate flat-series case. Source: ADR-128
FHIGHLOW window.

#### 2.95 Realized Volatility Cone (RVCONE — ADR-128)

Pulled from `research::get_rvcone`. Pure symbol-local HP stat that
computes 20d / 60d / 120d / 252d annualized realized volatility
(stdev of log returns × √252) and overlays the latest 20d RV
percentile against the rolling distribution of 20d RVs across the
full window. Header line gives the **cone_label** (COMPRESSED /
BELOW_AVG / TYPICAL / ELEVATED / EXTREME / INSUFFICIENT_DATA),
RV20, RV20 min/median/max, and percentile. Body block reports
RV20/RV60/RV120/RV252, RV20 rolling min/median/max, RV20
percentile, and bars used. Uses sample-mean stdev with N
denominator (matches RVOL convention). Latest 20d window is
excluded from the ranking distribution so it's compared against
its *history*. Source: ADR-128 RVCONE window.

#### 2.96 Calendar Period Breakdowns (CALPB — ADR-128)

Pulled from `research::get_calpb`. Pure symbol-local HP stat that
aligns to calendar boundaries rather than rolling-session offsets.
Computes MTD, QTD, current-year YTD, prior-quarter full return,
and prior-year full return. Complementary to PRICEPERF's rolling
1M/3M/6M/YTD/1Y lookbacks — portfolio reporting and reviews are
calendar-aligned. Header line gives the **momentum_label**
(ACCELERATING / STEADY / DECELERATING / REVERSING /
INSUFFICIENT_DATA, comparing QTD to prior-quarter with a 5pp
threshold), MTD, QTD, and prior-quarter. Body block reports MTD,
QTD, YTD, prior-quarter, prior-year, current year/quarter, and
latest close. Q1 prior-quarter correctly rolls to Q4 of prior
year. Source: ADR-128 CALPB window.

#### 2.97 Return Distribution Skewness (RETSKEW — ADR-129)

Pulled from `research::get_retskew`. Pure symbol-local HP stat over
the trailing 253-session window of log returns. Third standardized
moment (Fisher-Pearson) identifies asymmetry in the return
distribution: strong positive skew means rare large up-moves dominate,
strong negative skew means rare large down-moves dominate (typical
of crash-prone names). Header line gives **skew_label** (STRONG_LEFT
≤-1.0 / LEFT ≤-0.3 / SYMMETRIC / RIGHT / STRONG_RIGHT /
INSUFFICIENT_DATA). Body reports bars_used, mean/stdev of log
returns, skewness, positive-day share, and the largest single-day
up and down moves in the window. Source: ADR-129 RETSKEW window.

#### 2.98 Return Distribution Excess Kurtosis (RETKURT — ADR-129)

Pulled from `research::get_retkurt`. Pure symbol-local HP stat.
Fourth standardized moment minus 3 — excess kurtosis quantifies how
"fat-tailed" the return distribution is vs a normal (which has
kurtosis = 3). Also counts |z|>2 and |z|>3 outliers directly since
the tail-count is often more interpretable than the moment itself.
Header gives **kurt_label** (PLATYKURTIC ≤-0.5 / NORMAL <1.0 /
MILD_FAT <3.0 / FAT <6.0 / EXTREME_FAT / INSUFFICIENT_DATA). Body
reports bars_used, mean/stdev, excess kurtosis, and the 2σ/3σ
outlier counts plus the 2σ outlier rate (normal ≈ 4.55%).
Source: ADR-129 RETKURT window.

#### 2.99 Tail Ratio (TAILR — ADR-129)

Pulled from `research::get_tailr`. Pure symbol-local HP stat.
Non-parametric quantile-based view of tail asymmetry: tail_ratio =
95th percentile return / |5th percentile return|. Ratio > 1 →
upside tail dominates; < 1 → downside tail dominates. Complements
RETSKEW's moment-based view with a cleaner tail comparison that's
less sensitive to outliers. Header gives **bias_label**
(DOWNSIDE_HEAVY ≤0.6 / SLIGHT_DOWNSIDE / BALANCED / SLIGHT_UPSIDE /
UPSIDE_HEAVY / INSUFFICIENT_DATA). Body reports bars_used, P95,
P05, the 95/5 tail ratio, plus P99/P01 and the 99/1 extreme-tail
ratio. Source: ADR-129 TAILR window.

#### 2.100 Run Length (RUNLEN — ADR-129)

Pulled from `research::get_runlen`. Pure symbol-local HP stat. Mean
and longest runs of consecutive up-days and down-days. Long runs →
trending regime; short runs → choppy / mean-reverting. The
`current_run_length` field is signed (positive = in an up-run,
negative = in a down-run, 0 = flat) so the consumer can tell at a
glance whether the latest bar extends a streak. Header gives
**trend_label** (CHOPPY / MIXED / TRENDING / STRONG_TRENDING /
INSUFFICIENT_DATA) computed from the blend of average and longest
runs. Body reports bars_used, avg/longest up and down runs with
counts, and the signed current run. Source: ADR-129 RUNLEN window.

#### 2.101 Daily Range (DAYRANGE — ADR-129)

Pulled from `research::get_dayrange`. Pure symbol-local HP stat.
Average (high - low) / close over 60 sessions vs 252-session
baseline. Compression ratio = 60d avg / 252d avg: below 1 means
current 60d regime is tighter than the full-window baseline
("coiled" — breakout typically follows); above 1 means the name
is in an expanded/volatile regime. Header gives **range_label**
(TIGHT ≤0.75 / COMPRESSED ≤0.9 / NORMAL / EXPANDED <1.35 /
VERY_EXPANDED / INSUFFICIENT_DATA). Body reports bars_used,
avg 60d and 252d range %, latest bar's range %, compression
ratio, and the widest/narrowest range bars in the window.
Source: ADR-129 DAYRANGE window.

#### 2.102 Return Autocorrelation (AUTOCOR — ADR-131)

Pulled from `research::get_autocor`. Pure symbol-local HP stat over
the trailing 253-session window of log returns. Sample
autocorrelation of returns at lags 1 / 5 / 10 / 20 characterizes
serial dependence: lag-1 ACF > 0.15 → short-horizon momentum /
trend continuation; lag-1 ACF < -0.15 → short-horizon mean
reversion; |lag-1 ACF| < 0.05 → essentially random walk. The
multi-lag view catches weekly (lag-5) and monthly (lag-20)
seasonality that lag-1 alone would miss. Header gives
**regime_label** (MEAN_REVERTING ≤-0.15 / NEUTRAL / MOMENTUM ≥0.15 /
INSUFFICIENT_DATA). Body reports bars_used, mean log return, and
the four lag ACFs. Complements RSTATS / TECH (which report return
*level*) with a serial-dependence view that's the classical test
for whether "yesterday predicts today." Source: ADR-131 AUTOCOR
window.

#### 2.103 Hurst Exponent (HURST — ADR-131)

Pulled from `research::get_hurst`. Pure symbol-local HP stat.
Hurst exponent via rescaled-range (R/S) analysis — the canonical
long-memory / persistence statistic. R/S is computed at a family
of scales `[8, 12, 16, 24, 32, 48, 64, 96, 128]` (filtered so
each scale has ≥2 non-overlapping chunks), then H is the OLS
slope of `log(R/S_avg)` vs `log(scale)`. H ∈ [0, 1]: H<0.5 →
anti-persistent / mean-reverting, H≈0.5 → random walk / no
long memory, H>0.5 → persistent / trending. Header gives
**memory_label** (STRONG_MEAN_REVERT ≤0.35 / MEAN_REVERT ≤0.45 /
RANDOM_WALK / PERSISTENT ≥0.55 / STRONG_PERSISTENT ≥0.65 /
INSUFFICIENT_DATA). Body reports bars_used, H, scales fit,
smallest/largest scale used. Complements AUTOCOR (which measures
short-lag serial dependence) with a multi-scale persistence view
— two surfaces can agree (both mean-reverting or both trending)
or disagree (short-run momentum + long-run mean reversion is a
classic fat-tailed regime). Source: ADR-131 HURST window.

#### 2.104 Multi-Horizon Hit Rate (HITRATE — ADR-131)

Pulled from `research::get_hitrate`. Pure symbol-local HP stat.
Fraction of positive-return bars over the last 5 / 20 / 60 / 252
bars. Complements RSTATS (which reports mean return level) with
a frequency-of-winning view that doesn't care about magnitude.
Bullish when **both** short-horizon windows (h5 + h20) clear
55%; bearish when both sit below 45%. Also reports all-window
up / down / flat day counts for context. Header gives **hit_label**
(BEARISH / WEAK_BEARISH / NEUTRAL / WEAK_BULLISH / BULLISH /
INSUFFICIENT_DATA) computed from the blend of h20 and h60. Body
reports bars_used, up/down/flat counts, and the four window hit
rates. Useful for distinguishing "wins small + often" names from
"wins rarely but big" names — hit rate ≠ return sign.
Source: ADR-131 HITRATE window.

#### 2.105 Gain/Loss Asymmetry (GLASYM — ADR-131)

Pulled from `research::get_glasym`. Pure symbol-local HP stat.
Compares the typical *magnitude* of up-days vs down-days over
the trailing 253-session window, independent of which side has
more bars. `magnitude_ratio` = `avg_up_pct / avg_down_pct`: ratio
> 1.15 → upside asymmetry (up days tend to be bigger than down
days); < 0.85 → downside asymmetry (a crash-prone pattern).
Complements HITRATE (which counts wins) and RETSKEW (third
moment) with an average-magnitude view that's often easier to
read than skewness on fat-tailed names. Also reports medians
alongside means to show robustness to outliers. Header gives
**asymmetry_label** (DOWNSIDE_HEAVY ≤0.7 / SLIGHT_DOWNSIDE /
BALANCED / SLIGHT_UPSIDE / UPSIDE_HEAVY ≥1.3 / INSUFFICIENT_DATA).
Body reports bars_used, avg/median up & down pct magnitudes,
ratio, and the up/down day counts. Source: ADR-131 GLASYM window.

#### 2.106 Up/Down Volume Ratio (VOLRATIO — ADR-131)

Pulled from `research::get_volratio`. Pure symbol-local HP stat
over the trailing 253-session window. Ratio of average up-day
volume to average down-day volume: ratio > 1 means heavier
volume on up-days → classic accumulation pattern; < 1 means
heavier volume on down-days → distribution pattern. Header gives
**flow_label** (DISTRIBUTION ≤0.75 / SLIGHT_DISTRIBUTION /
NEUTRAL / SLIGHT_ACCUMULATION / ACCUMULATION ≥1.35 /
INSUFFICIENT_DATA). Body reports bars_used, up/down day counts,
ratio, avg/median up & down volume, and the single largest
up-day and down-day volume bars in the window. Gracefully emits
INSUFFICIENT_DATA when the HP cache was populated without volume
(some MT5 symbols have the volume field at 0) — the first broker
to populate volume on an LAN peer backfills the whole network.
Complements RSTATS / AUTOCOR / DAYRANGE (all price-only) with
the one volume-derived HP surface in the Round 23 bundle.
Source: ADR-131 VOLRATIO window.

#### 2.107 Rally History (DRAWUP — ADR-132)

Pulled from `research::get_drawup`. Pure symbol-local HP stat over the
trailing 253-session window. Upside mirror of DDHIST (ADR-127):
tracks running minimum close, reports the deepest peak-from-trough
advance `max_drawup_pct` (with trough date and peak date), the
longest rally duration in sessions, the count of local-trough-to-peak
advances ≥5% and ≥10%, and `current_drawup_pct` (latest close vs
running trough). Header gives **rally_label** (MUTED ≤5% / MILD ≤10% /
MEANINGFUL ≤20% / STRONG ≤50% / EXPLOSIVE >50% / INSUFFICIENT_DATA).
Complements DDHIST one-for-one: the pair of surfaces gives the full
peak-trough-peak-trough history view. Source: ADR-132 DRAWUP window.

#### 2.108 Overnight Gap Statistics (GAPSTATS — ADR-132)

Pulled from `research::get_gapstats`. Pure symbol-local HP stat — the
first surface in the packet to read `bar.open`. Iterates trailing-
window bar pairs computing `gap_t = (open_t - close_{t-1}) /
close_{t-1}`; a gap is "real" if |gap| > 0.5%. Header gives
**bias_label** (DOWN_BIAS ≤ -0.25% / SLIGHT_DOWN ≤ -0.1% / NEUTRAL /
SLIGHT_UP ≥ 0.1% / UP_BIAS ≥ 0.25% / INSUFFICIENT_DATA) computed
from the signed average gap. Body reports bars_used, gap_up / gap_down
counts, `gap_frequency_pct`, avg signed gap, avg up / down gap
magnitudes, and the single largest up and down gaps in the window.
Useful for reading whether a name has a systematic overnight skew
(often visible on earnings-heavy or news-driven names). Source:
ADR-132 GAPSTATS window.

#### 2.109 Volatility Clustering (VOLCLUSTER — ADR-132)

Pulled from `research::get_volcluster`. Pure symbol-local HP stat
over the trailing 253-session window. Canonical GARCH-effect test:
ACF of squared log returns and absolute log returns at lags 1 / 5 /
20. Header gives **cluster_label** (NONE ≤0.05 / MILD ≤0.15 /
MODERATE ≤0.3 / STRONG ≤0.5 / VERY_STRONG >0.5 / INSUFFICIENT_DATA)
bucketed from |r| lag-1 ACF, the most common reference metric. Body
reports bars_used, |r| ACF at lags 1 / 5 / 20, and r² ACF at lags
1 / 5 / 20. Complements AUTOCOR (ADR-131) one-to-one: return ACF
measures *directional* persistence, vol ACF measures *magnitude*
persistence. A name can have zero return ACF (random direction)
while still exhibiting strong volatility clustering ("big moves
follow big moves"). Source: ADR-132 VOLCLUSTER window.

#### 2.110 Close Placement (CLOSEPLC — ADR-132)

Pulled from `research::get_closeplc`. Pure symbol-local HP stat over
the trailing 253-session window. For each bar with `high > low`:
`pos = (close - low) / (high - low)` ∈ [0, 1]. Averaged over the
window, this captures bar "anatomy": near 1.0 → closes typically
pin near the high (buyers in control); near 0.0 → closes near the
low (sellers in control). Header gives **placement_label**
(STRONG_BEAR ≤0.35 / BEAR ≤0.45 / NEUTRAL / BULL ≥0.55 /
STRONG_BULL ≥0.65 / INSUFFICIENT_DATA). Body reports bars_used,
avg / median / latest placement, and the share of bars that closed
in the top 20% of the range (`pct_near_high`) and bottom 20%
(`pct_near_low`). Skips flat bars (`high == low`) to avoid divide-
by-zero. Source: ADR-132 CLOSEPLC window.

#### 2.111 Mean-Reversion Half-Life (MRHL — ADR-132)

Pulled from `research::get_mrhl`. Pure symbol-local HP stat over the
trailing 253-session window. Fits `r_t = α + β r_{t-1} + ε` to log
returns via two-pass OLS, then reports `half_life = -ln(2) / ln(β)`
for `0 < β < 1` — the explicit "how many sessions until a shock
decays to half its size" view. Header gives **regime_label**
(FAST_REVERT / MEAN_REVERTING half-life ≤10 / NEUTRAL / PERSISTENT
half-life ≥30 / STRONG_PERSISTENT half-life ≥60 / INSUFFICIENT_DATA).
Body reports bars_used, AR(1) β / α / R², and half-life in sessions.
Complements AUTOCOR (lag-k ACF) and HURST (multi-scale persistence)
one-for-one: AUTOCOR answers "is there lag-1 dependence?", HURST
answers "is there long-memory?", MRHL answers "how fast does a
shock decay?". β ≤ 0 → FAST_REVERT with half-life 0 (same-period
mean reversion); β ≥ 1 → INSUFFICIENT_DATA (explosive regime, should
not occur on stationary log returns). Source: ADR-132 MRHL window.

#### 2.112 Downside Deviation / Sortino (DOWNVOL — ADR-133)

Pulled from `research::get_downvol`. Pure symbol-local HP stat over the
trailing 253-session window. Iterates log returns accumulating
`down_sq = Σ min(r,0)²` and `up_sq = Σ max(r,0)²`, then reports
`downside_dev = √(down_sq / n)`, its annualized form `× √252`, the
symmetric upside deviation, Sortino ratio `mean / downside_dev` (both
raw and annualized), and `downside_pct_of_total` (what share of total
variance comes from down moves). Header gives **sortino_label**
(VERY_POOR annualized Sortino < -1 / POOR < 0 / NEUTRAL < 1 / GOOD < 2
/ EXCELLENT ≥ 2 / INSUFFICIENT_DATA). Complements RVCONE (total vol)
and TAILR (tail quantiles) with a pure downside-risk view. Source:
ADR-133 DOWNVOL window.

#### 2.113 Sharpe Ratio (SHARPR — ADR-133)

Pulled from `research::get_sharpr`. Pure symbol-local HP stat over the
trailing 253-session window. Classical `Sharpe = mean_return /
stdev_return` with rf = 0 (the HP cache doesn't carry a risk-free
series). Reports both raw daily and annualized (×√252) Sharpe, plus
annualized mean and stdev of returns. Header gives **sharpe_label**
(POOR < -0.5 / BELOW_AVG < 0.5 / NEUTRAL < 1 / GOOD < 2 / EXCELLENT
≥ 2 / INSUFFICIENT_DATA). The single canonical risk-adjusted return
scalar that every quantitative conversation starts with. Source:
ADR-133 SHARPR window.

#### 2.114 Kaufman Efficiency Ratio (EFFRATIO — ADR-133)

Pulled from `research::get_effratio`. Pure symbol-local HP stat over the
trailing 253-session window (requires ≥30 bars). `ER = |close_N -
close_1| / Σ |close_t - close_{t-1}|` — the share of gross price
travel that became net directional movement. Reports start/end closes,
net change (signed + pct), sum of absolute daily close changes, ER, and
`signed_efficiency = ER × sign(net_change)`. Header gives
**efficiency_label** (CHOP < 0.10 / NOISY < 0.25 / MIXED < 0.40 /
TRENDING < 0.60 / STRONG_TREND ≥ 0.60 / INSUFFICIENT_DATA). Sharper
single-number "trending vs chopping" signal than HURST. Source:
ADR-133 EFFRATIO window.

#### 2.115 Wick Bias (WICKBIAS — ADR-133)

Pulled from `research::get_wickbias`. Pure symbol-local HP stat over the
trailing 253-session window. For each bar with `high > low`:
`upper_wick = (high - max(o,c)) / range`, `lower_wick = (min(o,c) -
low) / range`, `body = 1 - upper - lower`. Reports average and median
upper/lower wick shares, average body share, and `bias_score =
avg_lower - avg_upper` (positive = buyers defending). Requires ≥20
non-flat bars. Header gives **bias_label** (SELLER_REJECT < -0.05 /
SELLER_LEAN < -0.02 / NEUTRAL ≤ 0.02 / BUYER_LEAN ≤ 0.05 /
BUYER_DEFEND > 0.05 / INSUFFICIENT_DATA). Natural partner to CLOSEPLC
— CLOSEPLC shows *where the bar closes* within its range; WICKBIAS
shows *how much of the range was rejection* above vs below. Source:
ADR-133 WICKBIAS window.

#### 2.116 Vol-of-Vol (VOLOFVOL — ADR-133)

Pulled from `research::get_volofvol`. Pure symbol-local HP stat over the
trailing 253-session window. Slides a 20-bar window over log returns
computing realized vol at each step, producing ≥30 rolling RV20 points
(requires ≥50 returns total). Reports mean / stdev / min / max / latest
RV20, and `CV = stdev(rv20) / mean(rv20)` (coefficient of variation).
Header gives **cv_label** (STABLE CV < 0.15 / MILD < 0.25 / MODERATE
< 0.40 / UNSTABLE < 0.60 / CHAOTIC ≥ 0.60 / INSUFFICIENT_DATA).
Captures "is the vol regime stable or does vol itself bounce?" —
complements VOLCLUSTER (which tests vol *autocorrelation*, not vol
*variability*). Source: ADR-133 VOLOFVOL window.

#### 2.117 Calmar Ratio (CALMAR — ADR-134)

Pulled from `research::get_calmar`. Pure symbol-local HP stat over the
trailing 253-session window. `calmar = annualized_return / max_drawdown`.
The canonical drawdown-adjusted return metric used in CTA / trend-following
evaluation. Reports total_return_pct, annualized_return_pct,
max_drawdown_pct, and calmar_ratio. Header gives **calmar_label**
(VERY_POOR <0.5 / POOR <1 / NEUTRAL <2 / GOOD <3 / EXCELLENT ≥3 or
zero-drawdown positive return / INSUFFICIENT_DATA). A monotonically
rising series (zero drawdown) gets EXCELLENT. Source: ADR-134 CALMAR window.

#### 2.118 Ulcer Index + Martin Ratio (ULCER — ADR-134)

Pulled from `research::get_ulcer`. Pure symbol-local HP stat over the
trailing 253-session window. `ulcer_index = sqrt(mean(dd_pct²))` where
`dd_pct = (close - running_peak) / running_peak × 100`. The continuous
drawdown-weighted risk measure that captures average pain, not just the
single worst event. Reports ulcer_index, mean/max drawdown %, share of
bars in drawdown, annualized return, and `martin_ratio =
annualized_return / ulcer_index` (the drawdown-analogue of Sharpe).
Header gives **ulcer_label** (LOW_PAIN <2 / MILD <5 / MODERATE <10 /
HIGH <20 / SEVERE ≥20 / INSUFFICIENT_DATA). Source: ADR-134 ULCER window.

#### 2.119 Lo-MacKinlay Variance Ratio (VARRATIO — ADR-134)

Pulled from `research::get_varratio`. Pure symbol-local HP stat over the
trailing 253-session window. `VR(q) = Var(q-period overlapping returns) /
(q × Var(1-period returns))`. VR=1 for random walk, >1 for trending, <1
for mean-reverting. The first formal random-walk *hypothesis test* in the
packet — unlike HURST/AUTOCOR which are descriptive statistics, VARRATIO
has z-statistics with known asymptotic distributions. Reports VR at
horizons 2/5/10/20 plus z-stats for horizons 2 and 5. Header gives
**rw_label** (STRONG_REVERT VR5 <0.7 / MEAN_REVERT <0.9 / RANDOM_WALK
0.9–1.1 / TRENDING <1.3 / STRONG_TREND ≥1.3 / INSUFFICIENT_DATA).
Requires ≥40 log returns. Source: ADR-134 VARRATIO window.

#### 2.120 Amihud Illiquidity (AMIHUD — ADR-134)

Pulled from `research::get_amihud`. Pure symbol-local HP stat over the
trailing 253-session window. `ILLIQ = mean(|r_t| / (close_t × volume_t))
× 1e6` — the canonical Amihud (2002) microstructure liquidity scalar.
Higher = less liquid = more price impact per dollar traded. Reports
mean/median/90th-percentile of the daily ILLIQ series plus average daily
dollar volume. Header gives **illiq_label** (VERY_LIQUID <0.01 / LIQUID
<0.1 / MODERATE <1 / ILLIQUID <10 / VERY_ILLIQUID ≥10 /
INSUFFICIENT_DATA). Requires ≥20 valid bar pairs (non-zero dollar
volume). Source: ADR-134 AMIHUD window.

#### 2.121 Jarque-Bera Normality Test (JBNORM — ADR-134)

Pulled from `research::get_jbnorm`. Pure symbol-local HP stat over the
trailing 253-session window. `JB = (n/6)(S² + K²/4)` where S = sample
skewness and K = excess kurtosis. Under H₀ (normality), JB ~ χ²(2),
so `p = exp(-JB/2)` (exact). Combines RETSKEW + RETKURT into a single
"can we reject normality?" answer. Header gives **normal_label** (NORMAL
p >0.10 / MILD_DEPARTURE >0.05 / MODERATE_DEPARTURE >0.01 / NON_NORMAL
>0.001 / STRONGLY_NON_NORMAL ≤0.001 / INSUFFICIENT_DATA). The first
surface in the packet to report an explicit p-value. Source: ADR-134
JBNORM window.

#### 2.122 Omega Ratio at τ=0 (OMEGA — ADR-135)

Pulled from `research::get_omega`. Pure symbol-local HP stat over the
trailing 253-session window. `Ω(τ) = E[max(r-τ,0)] / E[max(τ-r,0)]`
partitions the *full* return distribution at threshold τ into gains
and losses. At τ=0: gains-sum / losses-sum. Distribution-free — unlike
Sharpe (moment-based), DOWNVOL (variance-based), or CALMAR (max-dd
only), Omega uses the entire distributional shape without any moment
assumption, so it is robust on fat-tailed or asymmetric series where
moment metrics can mislead. Header gives **omega_label** (VERY_POOR
<0.5 / POOR <0.9 / NEUTRAL <1.1 / GOOD <1.5 / EXCELLENT ≥1.5 or ∞ /
INSUFFICIENT_DATA). Body reports gains_sum, losses_sum, gain_days,
loss_days, win_rate_pct. Source: ADR-135 OMEGA window.

#### 2.123 Detrended Fluctuation Analysis (DFA — ADR-135)

Pulled from `research::get_dfa`. Pure symbol-local HP stat over the
trailing 253-session window (minimum 100 returns). Computes the
Peng-style DFA exponent α: form the cumulative-sum profile of
demeaned log-returns, for each scale s split the profile into
non-overlapping windows, linearly detrend each, RMS the residuals to
get F(s), and OLS-regress log(F(s)) on log(s). The slope is α — a
Hurst-exponent analogue that is **robust to non-stationarity** where
R/S-based HURST (Round 22) is not. α ≈ 0.5 random walk; α > 0.5
persistent (trending); α < 0.5 anti-persistent (mean-reverting).
Header gives **dfa_label** (ANTI_PERSISTENT <0.35 / MEAN_REVERTING
<0.45 / RANDOM_WALK <0.55 / PERSISTENT <0.65 / STRONGLY_PERSISTENT
≥0.65 / INSUFFICIENT_DATA). Body reports alpha, num_scales (distinct
window sizes sampled), and log-log R². Cross-check: if DFA α
disagrees meaningfully with HURST H, the series is non-stationary
enough to invalidate R/S. Source: ADR-135 DFA window.

#### 2.124 Burke Ratio (BURKE — ADR-135)

Pulled from `research::get_burke`. Pure symbol-local HP stat over the
trailing 253-session window. `Burke = annualized_return / sqrt(Σ dd_i²)`
summed over **distinct** drawdown *events* (peak→trough→recovery
episodes). Sits between CALMAR (only the deepest drawdown counts) and
ULCER (every bar of every drawdown counts continuously). Burke weights
by the top-k worst completed episodes, which is what practitioners
actually care about — "how bad are my worst 3 drawdowns, not just the
single deepest one?" Header gives **burke_label** (VERY_POOR <-0.5 /
POOR <0 / NEUTRAL <0.5 / GOOD <1.5 / EXCELLENT ≥1.5, and EXCELLENT on
no-event with positive return / INSUFFICIENT_DATA). Body reports
annualized_return_pct, dd_event_count (number of completed drawdown
episodes), sum_sq_drawdowns, worst_event_dd_pct. Source: ADR-135
BURKE window.

#### 2.125 Monthly Seasonality (MONTHSEAS — ADR-135)

Pulled from `research::get_monthseas`. Pure symbol-local calendar-axis
statistic — uniquely in the packet, uses the **full HP cache** (not
the 253-bar window) because meaningful seasonality requires multi-year
history. For each calendar month (Jan–Dec), reports hit rate (share of
historical years the month closed positive) and mean close-to-close
month return. Captures "Sell in May," "Santa rally," "January effect,"
"summer doldrums," and similar calendar patterns that no other packet
surface sees. Header gives **season_label** (STRONG_SEASONAL best-worst
hit spread ≥40% / MILD_SEASONAL ≥25% / NEUTRAL ≥15% / INCONSISTENT
<15% / INSUFFICIENT_DATA). Body identifies best_month and worst_month
by index + hit rate + mean return, and emits a full 12-cell month
grid. The calendar axis adds a complementary orthogonal view that
lets an agent detect when a ticker has unusually strong or weak
calendar effects vs its peer set. Source: ADR-135 MONTHSEAS window.

#### 2.126 Roll Implicit Bid-Ask Spread (ROLLSPRD — ADR-135)

Pulled from `research::get_rollsprd`. Pure symbol-local HP stat over
the trailing 253-session window. Roll (1984) exploits the fact that
bid/ask bounce induces negative first-lag autocorrelation in
consecutive price changes: `spread = 2·√(-Cov(Δp_t, Δp_{t-1}))`. When
the first-lag covariance is negative (bid/ask bounce dominates), Roll
gives a clean closed-form effective spread in bps. When covariance is
non-negative (trending dominates), the model fails identifiably — we
report the `INVALID_POSITIVE_COV` label rather than a bogus spread,
which is itself information (the ticker is in a regime where
microstructure noise does not dominate daily returns). Microstructure
companion to AMIHUD (Round 26): AMIHUD captures price impact per
dollar traded, ROLLSPRD captures the implicit effective spread.
Header gives **roll_label** (TIGHT <10 bps / NORMAL <30 / WIDE <75 /
VERY_WIDE ≥75 / INVALID_POSITIVE_COV / INSUFFICIENT_DATA). Body
reports first_lag_cov, mean_price, implicit_spread, and
implicit_spread_bps. Source: ADR-135 ROLLSPRD window.

#### 2.127 Parkinson H-L Volatility (PARKINSON — ADR-136)

Pulled from `research::get_parkinson`. Parkinson (1980)
range-based volatility estimator `σ² = (1/(4·ln2·n)) · Σ(ln(H/L))²`
computed over the trailing 253-session window on the HP cache.
~5.2× more statistically efficient than close-to-close vol by
virtue of using the daily High-Low range. Header reports regime
bucket (VERY_LOW <10% / LOW <20% / NORMAL <40% / HIGH <60% /
VERY_HIGH ≥60% annualized σ / INSUFFICIENT_DATA). Body reports
daily_vol_pct, annualized_vol_pct, and mean_hl_log_ratio.
Source: ADR-136 PARKINSON window.

#### 2.128 Garman-Klass OHLC Volatility (GKVOL — ADR-136)

Pulled from `research::get_gkvol`. Garman-Klass (1980) OHLC
volatility estimator `σ² = (1/n)·Σ[0.5·(ln H/L)² - (2ln2-1)·(ln C/O)²]`
over the trailing 253-session window. Combines the H-L range with
the C-O drift component for ~7.4× efficiency — the most commonly
deployed range-vol estimator in practice. Header reports the same
regime buckets as PARKINSON. Body reports daily_vol_pct,
annualized_vol_pct, range_component, and co_component (making the
two contributions visible separately). Source: ADR-136 GKVOL
window.

#### 2.129 Rogers-Satchell Drift-Free Volatility (RSVOL — ADR-136)

Pulled from `research::get_rsvol`. Rogers-Satchell (1991)
drift-independent OHLC vol estimator
`σ² = (1/n)·Σ[ln(H/C)·ln(H/O) + ln(L/C)·ln(L/O)]` over the
trailing 253-session window. Unlike PARKINSON and GKVOL (which
assume zero drift), RSVOL is **unbiased under non-zero drift** —
so a large gap between GKVOL and RSVOL annualized σ is itself a
signal that the window has material drift. Header reports the
same regime buckets as PARKINSON/GKVOL. Body reports
daily_vol_pct and annualized_vol_pct. Source: ADR-136 RSVOL
window.

#### 2.130 Conditional VaR / Expected Shortfall (CVAR — ADR-136)

Pulled from `research::get_cvar`. CVaR / Expected Shortfall at 5%
and 1% levels computed over the trailing 253-session window on the
HP cache. VaR(p) is the p-th percentile of daily log returns; CVaR
is the **mean** of returns ≤ VaR — the coherent downside-risk
measure preferred by Basel III (satisfies subadditivity, which
plain VaR does not). Distinct from TAILR (quantile ratio — shape)
and DOWNVOL (variance of negative returns — scale): CVAR answers
"given we're in the worst 5% of days, what's the *average* loss?"
Header reports regime bucket (MINIMAL |ES(5%)|<1% / LOW <2.5% /
MODERATE <5% / HIGH <10% / EXTREME ≥10% / INSUFFICIENT_DATA). Body
reports var_5pct_ret_pct, cvar_5pct_ret_pct, var_1pct_ret_pct,
cvar_1pct_ret_pct, tail_days_5pct, and tail_days_1pct. Source:
ADR-136 CVAR window.

#### 2.131 Day-of-Week Seasonality (DOWEFFECT — ADR-136)

Pulled from `research::get_doweffect`. Day-of-week intraday (O→C)
seasonality hit rate + mean return computed over the **full** HP
cache (not 253-windowed). For each weekday (Mon-Fri) reports hit
rate (share of that weekday which closed positive intraday) and
mean intraday % return. Companion to MONTHSEAS on the weekday axis:
captures Monday-effect, Friday-rally, Wednesday-weakness etc.
backed by long-horizon academic literature (French 1980, Ariel
1987). Header reports regime bucket (STRONG_EFFECT best-worst hit
spread ≥20% / MILD_EFFECT ≥10% / NEUTRAL ≥5% / INCONSISTENT <5% /
INSUFFICIENT_DATA). Body reports best/worst weekday, hit_pct and
mean_ret_pct per weekday, sample counts per weekday, and weeks
covered. Source: ADR-136 DOWEFFECT window.

#### 2.132 Sterling Ratio (STERLING — ADR-137)

Pulled from `research::get_sterling`. Annualized return divided by
the arithmetic mean of the **N worst distinct drawdown events**
(canonical N=5) over the trailing 253-session window. Textbook
middle ground between CALMAR (single worst dd — fragile to
one-off tail outliers) and BURKE (sum-of-squared drawdowns —
quadratic penalty that over-weights clusters). Large
CALMAR/STERLING gap ⇒ one outlier drawdown dominated; CALMAR ≈
STERLING ⇒ top-5 drawdowns are similar magnitude. Header reports
regime bucket (VERY_POOR ratio<−0.5 / POOR <0 / NEUTRAL <0.5 /
GOOD <1.5 / EXCELLENT ≥1.5 / INSUFFICIENT_DATA). Body reports
bars_used, annualized_return_pct, worst_n, dd_event_count,
mean_worst_dd_pct, and sterling_ratio. Source: ADR-137 STERLING
window.

#### 2.133 Kelly Fraction (KELLYF — ADR-137)

Pulled from `research::get_kellyf`. Classical Kelly position-sizing
scalar `f* = (b·p − q) / b` where p=win rate, q=1−p,
b=avg_win/avg_loss over the trailing 253-session window (log
returns converted back to simple %). First packet surface that is
forward-looking (optimization target) rather than realized
risk-adjusted performance. Pairs with CALMAR/BURKE/STERLING:
those answer "how did it perform?"; KELLYF answers "how much
weight should it get?". Header reports regime bucket
(SKIP f*≤0 / MARGINAL <0.10 / MODERATE <0.25 / AGGRESSIVE <0.50
/ ALL_IN ≥0.50 / INSUFFICIENT_DATA). Practitioners typically
use `half_kelly` (f*/2); ALL_IN is usually a sign of sample
noise, not a real signal. Body reports win_rate, loss_rate,
avg_win_pct, avg_loss_pct, win_loss_ratio, kelly_fraction, and
half_kelly. Source: ADR-137 KELLYF window.

#### 2.134 Ljung-Box Joint Autocorrelation (LJUNGB — ADR-137)

Pulled from `research::get_ljungb`. Portmanteau Q-statistic
`Q = n(n+2)·Σ(ρ_k²/(n−k))` for k=1..h (h=10) testing whether
returns are white noise. Under the null `Q ~ χ²(h)`, so a small
p-value rejects the "returns are uncorrelated across lags 1..h"
hypothesis — the canonical econometrics test for model adequacy.
Complements AUTOCOR (per-lag ρ_k at k=1/5/10/20 — four separate
numbers) with a single joint-significance p-value. Header
reports regime bucket (WHITE_NOISE p≥0.10 / WEAK_DEP ≥0.05 /
MODERATE_DEP ≥0.01 / STRONG_DEP <0.01 / INSUFFICIENT_DATA).
Body reports bars_used, lag_h, q_statistic, p_value, and
reject_white_noise. P-value uses Wilson-Hilferty cube-root
approximation to χ²(h) — accurate at label-bucket granularity;
documented in ADR-137. Source: ADR-137 LJUNGB window.

#### 2.135 Wald-Wolfowitz Runs Test (RUNSTEST — ADR-137)

Pulled from `research::get_runstest`. Formal inferential test of
the sign sequence: given n₁ positive-return days and n₂ negative
among n signed days, the number of runs has null mean
`2n₁n₂/n + 1` and variance `2n₁n₂(2n₁n₂−n)/(n²(n−1))`. A
z-statistic and two-sided p-value reject (or not) the null of
random sign ordering. Complements RUNLEN (descriptive —
longest/mean streak) with inferential significance. Header
reports regime bucket (RANDOM / ANTI_CLUST z>0 reject / 
SLIGHT_CLUST z<0 p≥0.01 / MOD_CLUST p≥0.001 / STRONG_CLUST
p<0.001 / INSUFFICIENT_DATA). Body reports bars_used,
positive_days, negative_days, runs_observed, runs_expected,
runs_std, z_statistic, p_value, and reject_randomness. P-value
uses Abramowitz & Stegun 7.1.26 rational approximation to the
standard normal CDF. Source: ADR-137 RUNSTEST window.

#### 2.136 Zero-Return-Day Fraction (ZERORET — ADR-137)

Pulled from `research::get_zeroret`. Lesmond-Ogden-Trzcinka
(1999) liquidity proxy: fraction of bars with
`|log_return| < 1e-6` (i.e. exactly unchanged close) over the
trailing 253-session window. Illiquid securities show more
zero-return days (dealers don't update the close because nobody
traded). Third foundational microstructure scalar, distinct from
AMIHUD (price impact per $ of volume) and ROLLSPRD (implicit
bid-ask spread). Together AMIHUD + ROLLSPRD + ZERORET cover the
three canonical academic liquidity proxies. Header reports
regime bucket (HIGHLY_LIQUID <1% / LIQUID <5% / MODERATE <15% /
ILLIQUID <30% / VERY_ILLIQUID ≥30% / INSUFFICIENT_DATA). Body
reports bars_used, zero_day_count, zero_day_pct,
longest_zero_streak, and epsilon. Source: ADR-137 ZERORET window.

#### 2.137 Probabilistic Sharpe Ratio (PSR — ADR-138)

Pulled from `research::get_psr`. Lopez de Prado (2012):
`PSR = Φ((SR − SR*)·√(n−1) / √(1 − γ₃·SR + (γ₄−1)/4·SR²))` —
the probability that the true Sharpe exceeds a benchmark SR*,
given sample size, sample skewness γ₃ and kurtosis γ₄. Corrects
SHARPR for non-normal return distributions; unlike SHARPR which
reports a magnitude, PSR reports a *probability*. A ticker with
SR=1.2 and PSR=0.48 is statistically indistinguishable from
noise, while the same SR with PSR=0.96 is a strong signal.
First packet surface to apply higher-moment correction to a
return-quality ratio. Header reports regime bucket
(VERY_LOW <0.50 / LOW <0.75 / MODERATE <0.90 / HIGH <0.95 /
VERY_HIGH ≥0.95 / INSUFFICIENT_DATA). Body reports bars_used,
annualized Sharpe, skewness, kurtosis, SR benchmark, and PSR.
Source: ADR-138 PSR window.

#### 2.138 Dickey-Fuller Unit-Root Test (ADF — ADR-138)

Pulled from `research::get_adf`. Regresses
`Δlog(p)_t = α + β·log(p)_{t-1} + ε` and reports
`t-stat = β̂/se(β̂)` against Dickey-Fuller critical values
(MacKinnon 1996 constant-only): {−3.43, −2.86, −2.57} at
1/5/10%. A sufficiently-negative t-statistic rejects the unit
root, implying the log-price series is stationary / mean-
reverting. Complements HURST (continuous persistence) and DFA
(noise-robust persistence) with a binary reject/no-reject
inferential statement. Header reports regime bucket
(STATIONARY / BORDERLINE / NON_STATIONARY / INSUFFICIENT_DATA).
Body reports bars_used, β, SE(β), t-statistic, the three
critical values, and reject_unit_root. Source: ADR-138 ADF
window.

#### 2.139 Mann-Kendall Trend Test (MNKENDALL — ADR-138)

Pulled from `research::get_mnkendall`. Nonparametric trend-
presence test: `S = Σᵢ<ⱼ sign(x_j − x_i)` over all pairs of
log-prices, with closed-form null variance
`Var(S) = n(n−1)(2n+5)/18` (no ties correction). Distribution-
free — unlike linear regression, it makes no assumption of
linearity or normality, just tests whether observation *order*
reflects a monotone trend. Pairs with ADF: ADF tests
stationarity (unit root); MNKENDALL tests trend presence;
together they distinguish drifting-and-stationary from
drifting-and-non-stationary series. Header reports regime
bucket (STRONG_UP / UP / NO_TREND / DOWN / STRONG_DOWN /
INSUFFICIENT_DATA). Body reports bars_used, S-statistic,
variance, z-statistic, p-value, Kendall τ, and
reject_no_trend. Source: ADR-138 MNKENDALL window.

#### 2.140 Bipower Variation / Jump Ratio (BIPOWER — ADR-138)

Pulled from `research::get_bipower`. Barndorff-Nielsen & Shephard
(2004) decomposition of realized variance: `BPV = (π/2)·Σ|r_t|·|r_{t-1}|`
is a consistent estimator of the integrated *continuous*
variance, so `1 − BPV/RV` is the share of realized variance
attributable to jumps (discrete events). Distinct from the
volatility-magnitude estimators (CLOSEVOL/PARKINSON/GKVOL/RSVOL/
VOLOFVOL) — BIPOWER is a *composition* metric that separates
diffusive from jump components. Useful for regime classification:
heavy-jumps returns need different risk modeling than diffusive.
Header reports regime bucket (NO_JUMPS <5% / MILD_JUMPS <20% /
NOTABLE_JUMPS <40% / HEAVY_JUMPS ≥40% / INSUFFICIENT_DATA). Body
reports bars_used, realized variance, bipower variation,
annualized continuous vol %, annualized realized vol %, jump
ratio, and jump %. Source: ADR-138 BIPOWER window.

#### 2.141 Drawdown Duration Statistics (DDDUR — ADR-138)

Pulled from `research::get_dddur`. Walks the closing-price series
with a running-max tracker and records, for each closed drawdown
event, the peak-to-recovery bar count. Complements the
magnitude-focused drawdown trio (CALMAR single worst / BURKE
sum-of-squares / STERLING mean of N worst) with a *duration*
axis: "how long am I underwater?". Header reports regime bucket
(MOSTLY_DRY <20% / FREQUENT_DD <40% / PERSISTENT_DD <60% /
DEEP_WATER ≥60% / INSUFFICIENT_DATA). Body reports bars_used,
dd_event_count (closed drawdowns), max/mean/median event
durations in bars, total bars underwater, percentage of time
underwater, currently_underwater flag, and current_dd_duration
(if a drawdown is still open at window end). Source: ADR-138
DDDUR window.

#### 2.142 Hill Tail-Index Estimator (HILLTAIL — ADR-139)

Pulled from `research::get_hilltail`. Pure symbol-local HP stat
over the trailing 253-session window. For order statistics
X_(1) ≥ X_(2) ≥ … ≥ X_(n) of |r_t|, the Hill estimator
`α̂ = k / Σᵢ₌₁ᵏ log(X_(i) / X_(k+1))` with k ≈ 10%·n estimates
the Pareto tail exponent `P(|R| > x) ≈ c·x^(−α)`. Small α (≤2)
⇒ infinite-variance tails; α > 4 ≈ Gaussian-like tails.
Complements JBNORM (joint normality test) and KURT (fourth-moment
magnitude — which becomes meaningless when γ₄ is infinite) with
a distribution-free power-law exponent. Separate estimates on
left-tail (negative-return magnitudes) and right-tail
(positive-return magnitudes) expose tail asymmetry invisible to
KURT. Header gives **tail_label** (GAUSSIAN_LIKE α>4 / LIGHT_TAIL
α>3 / MODERATE_TAIL α>2 / HEAVY_TAIL α>1 / VERY_HEAVY_TAIL α≤1 /
INSUFFICIENT_DATA). Body reports bars_used, k_order_stats,
threshold_abs = X_(k+1), and the three α estimates. Source:
ADR-139 HILLTAIL window.

#### 2.143 Engle ARCH-LM Test (ARCHLM — ADR-139)

Pulled from `research::get_archlm`. Pure symbol-local HP stat
over the trailing 253-session window. Engle (1982) regresses
squared mean-residuals ε²_t on intercept + ε²_{t-1}, …,
ε²_{t-5} and reports `LM = n·R² ~ χ²(5)` under H₀ (no
conditional heteroskedasticity). Critical values χ²₀.₀₅(5)=11.07
and χ²₀.₀₁(5)=15.09 are hardcoded. p-value is computed via the
Wilson-Hilferty `χ² → Φ` transform for display; the label uses
direct critical-value comparison. Complements VOLOFVOL
(descriptive rolling-σ scatter) with the canonical formal test
for volatility clustering. Joins LJUNGB / RUNSTEST / ADF /
MNKENDALL / CUSUM as the sixth inferential diagnostic and the
first on *second-moment* memory. Header gives **arch_label**
(NO_ARCH <11.07 / WEAK_ARCH <15.09 / STRONG_ARCH ≥15.09 /
INSUFFICIENT_DATA; singular design ⇒ NO_ARCH). Body reports
bars_used, q_lags=5, r_squared, lm_statistic, p_value,
critical values, and `reject_homoskedastic`. Source: ADR-139
ARCHLM window.

#### 2.144 Pain Index + Pain Ratio (PAINRATIO — ADR-139)

Pulled from `research::get_painratio`. Pure symbol-local HP stat
over the trailing 253-session window. **Pain Index** = arithmetic
mean of |dd_t| (%) over every bar, where `dd_t = (close_t −
peak_t)/peak_t·100`. **Pain Ratio** = `annualized_return /
pain_index` — the drawdown-averaged analogue of
Sharpe/Calmar/Burke/Ulcer/Sterling. Completes the magnitude-
norm sextet alongside CALMAR (sup / max dd), BURKE (L² /
√Σdd²), STERLING (mean of worst N), ULCER (RMS / √mean(dd²)),
and DDDUR (duration). PAIN is the L¹ norm — treats every bar
equally instead of weighting the worst. Header gives
**pain_label** (LOW_PAIN <1% / MILD_PAIN <3% / MODERATE_PAIN
<7% / HIGH_PAIN <15% / SEVERE_PAIN ≥15% / INSUFFICIENT_DATA).
Body reports bars_used, pain_index_pct, annualized_return_pct,
pain_ratio, and max_dd_pct (companion magnitude). Source:
ADR-139 PAINRATIO window.

#### 2.145 Brown-Durbin-Evans CUSUM Break Test (CUSUM — ADR-139)

Pulled from `research::get_cusum`. Pure symbol-local HP stat
over the trailing 253-session window. Builds standardized
cumulative sum `S_t = Σ_{s=1..t} (r_s − r̄) / σ̂` and reports
`D = max_t |S_t| / √n`, which under H₀ (mean stability) has the
Kolmogorov-Smirnov limiting distribution with critical values
{10%=1.22, 5%=1.36, 1%=1.63}. Rejection ⇒ structural break in
the return mean somewhere within the window. First structural-
break test in the packet; pairs with ADF (stationarity of
levels), LJUNGB (joint autocorrelation), RUNSTEST (sign
randomness), ARCHLM (second-moment memory), and MNKENDALL
(trend presence) as the sixth inferential diagnostic covering
generator-stability. Header gives **cusum_label** (STABLE
<1.22 / MARGINAL <1.36 / BREAK_DETECTED <1.63 / STRONG_BREAK
≥1.63 / INSUFFICIENT_DATA). Body reports bars_used, max_abs_cusum,
test_statistic (the scaled D), max_abs_bar (index of the extreme),
direction_at_max ("UP"/"DOWN"/"NONE"), critical values, and
`reject_stability`. Source: ADR-139 CUSUM window.

#### 2.146 Cornish-Fisher Modified VaR (CFVAR — ADR-139)

Pulled from `research::get_cfvar`. Pure symbol-local HP stat
over the trailing 253-session window. Applies the Cornish-Fisher
(1938) expansion
`z* = z + (z²−1)·γ₃/6 + (z³−3z)·γ₄/24 − (2z³−5z)·γ₃²/36` to
the standard-normal quantile z (z=-1.645 for 5%, z=-2.326 for
1%), then reports `CF-VaR = μ + z*·σ`. Corrects the Gaussian
parametric VaR quantile for sample skewness (γ₃) and *excess*
kurtosis (γ₄). Complements CVAR (empirical tail mean — fully
nonparametric) with a smooth analytical quantile an agent can
extrapolate beyond the worst observed sample loss. Distinct
from TAILR (shape) and DOWNVOL (scale). Header gives
**cfvar_label** (BENIGN |Δ/Gauss|<10% / SKEW_DRIVEN
skew-term dominant 10-50% / KURT_DRIVEN kurt-term dominant
10-50% / EXTREME_DEVIATION |Δ/Gauss|>50% / INSUFFICIENT_DATA).
Body reports bars_used, mean_ret_pct, sigma_ret_pct, skewness,
excess_kurtosis, gauss_var_5pct_pct, cf_var_5pct_pct,
gauss_var_1pct_pct, cf_var_1pct_pct, cf_adjustment_5pct_pct,
skew_term_5pct, and kurt_term_5pct. Source: ADR-139 CFVAR window.

#### 2.147 Shannon Return Entropy (ENTROPY — ADR-140)

Pulled from `research::get_entropy`. Computes Shannon entropy
H = −Σ pᵢ log₂(pᵢ) over a histogram of daily log-returns with
bins = ceil(√n). Normalised entropy H/H_max ∈ [0,1] enables
cross-symbol comparison. Low entropy ⇒ concentrated / predictable
returns; high entropy ⇒ dispersed / unpredictable. First
information-theoretic distributional measure in the packet —
orthogonal to moment-based (KURT, SKEW) and test-based (JBNORM)
diagnostics. Header gives **entropy_label** (LOW_ENTROPY normalised
< 0.50 / MODERATE_ENTROPY < 0.70 / HIGH_ENTROPY < 0.85 /
VERY_HIGH_ENTROPY ≥ 0.85 / INSUFFICIENT_DATA). Body reports
bars_used, num_bins, entropy_bits, max_entropy_bits, and
normalised_entropy. Source: ADR-140 ENTROPY window.

#### 2.148 Rachev Ratio (RACHEV — ADR-140)

Pulled from `research::get_rachev`. Computes the Rachev ratio =
ES_α(+R) / ES_α(−R) — ratio of right-tail expected gain to
left-tail expected loss at matching confidence levels (5% and 1%).
Rachev > 1 ⇒ upside tail outweighs downside tail. Complements
TAILR (quantile ratio), CVAR (left-tail ES only), and CFVAR
(parametric moment-adjusted). Header gives **rachev_label**
(STRONG_LEFT_TAIL R₅% < 0.5 / LEFT_HEAVY < 0.8 / SYMMETRIC
0.8–1.2 / RIGHT_HEAVY > 1.2 / STRONG_RIGHT_TAIL > 2.0 /
INSUFFICIENT_DATA). Body reports bars_used, es_right_5pct,
es_left_5pct, rachev_5pct, es_right_1pct, es_left_1pct, and
rachev_1pct. Source: ADR-140 RACHEV window.

#### 2.149 Gain-to-Pain Ratio (GPR — ADR-140)

Pulled from `research::get_gpr`. Computes GPR = Σ rₜ / Σ |min(rₜ,0)|
(Schwager) — net return per unit of total realized loss. Also reports
Profit Factor = Σ max(rₜ,0) / Σ |min(rₜ,0)| = GPR + 1 (gross gain
per gross loss). Distinct axis from Pain Ratio (drawdown-based) and
Omega (threshold integration). Header gives **gpr_label** (DEEP_PAIN
GPR < −0.5 / NEGATIVE < 0 / MODEST < 0.5 / GOOD < 1.5 / EXCELLENT
≥ 1.5 / INSUFFICIENT_DATA). Body reports bars_used,
sum_all_returns_pct, sum_gains_pct, sum_losses_pct, gain_to_pain,
profit_factor, win_count, and loss_count. Source: ADR-140 GPR window.

#### 2.150 Partial Autocorrelation (PACF — ADR-140)

Pulled from `research::get_pacf`. Computes partial autocorrelation at
lags 1–5 via the Durbin-Levinson recursion on the sample ACF. Reports
individual PACF values plus Bartlett 95% critical band ±1.96/√n.
Decomposes the joint autocorrelation tested by LJUNGB into lag-specific
contributions — tells an agent whether lag-1 mean reversion, lag-2
momentum, or longer-lag calendar effects are present. Header gives
**pacf_label** (NO_STRUCTURE none significant / LAG1_DOMINANT only
lag 1 / LAG_STRUCTURE multiple / STRONG_STRUCTURE max |PACF| > 2×
critical / INSUFFICIENT_DATA). Body reports bars_used, pacf_lag1..5,
bartlett_crit_95, significant_lags, max_abs_pacf, and max_abs_lag.
Source: ADR-140 PACF window.

#### 2.151 Approximate Entropy (APEN — ADR-140)

Pulled from `research::get_apen`. Computes approximate entropy
(Pincus 1991) with m=2, r=0.2·σ — measures regularity /
predictability of the return time series. Low ApEn ⇒ regular,
self-similar patterns (returns repeat); high ApEn ⇒ irregular,
complex dynamics. Captures nonlinear short-range structure invisible
to HURST (long-range), DFA (trend), LJUNGB (linear), and RUNSTEST
(sign-only). ApEn is clamped to max(0, φ^m − φ^{m+1}) to handle
self-match edge effects. Header gives **apen_label** (REGULAR ApEn
< 0.3 / MODERATE < 0.7 / COMPLEX < 1.2 / HIGHLY_COMPLEX ≥ 1.2 /
INSUFFICIENT_DATA). Body reports bars_used, embed_dim, tolerance,
phi_m, phi_m1, and apen. Source: ADR-140 APEN window.

#### 2.152 Upside Potential Ratio (UPR — ADR-141)

Pulled from `research::get_upr`. Computes the Sortino & van der Meer
(1991) Upside Potential Ratio = UPM₁(MAR) / √LPM₂(MAR) with MAR=0.
UPM₁ = mean of max(r,0), LPM₂ = mean of min(r,0)². Separates upside
capture from downside risk — distinct from Sharpe (total vol), Sortino
(downside dev only), and Omega (threshold integration). Header gives
**upr_label** (POOR UPR < 0.3 / BELOW_AVERAGE < 0.6 / AVERAGE < 1.0 /
GOOD < 1.5 / EXCELLENT ≥ 1.5 / INSUFFICIENT_DATA). Body reports
bars_used, upm1, lpm2, sqrt_lpm2, upr, up_days, down_days.
Source: ADR-141 UPR window.

#### 2.153 Leverage Effect (LEVEREFF — ADR-141)

Pulled from `research::get_levereff`. Computes the Black (1976)
leverage effect: corr(rₜ, rₜ₊₁²) plus asymmetric vol ratio
(down-vol / up-vol). Strong negative corr ⇒ negative returns amplify
future volatility. Header gives **levereff_label** (STRONG_INVERSE
corr < −0.3 / MODERATE_INVERSE < −0.1 / WEAK_OR_NONE −0.1–0.1 /
POSITIVE_LEVERAGE ≥ 0.1 / INSUFFICIENT_DATA). Body reports bars_used,
corr_r_vol, down_vol, up_vol, asym_ratio, pairs_used.
Source: ADR-141 LEVEREFF window.

#### 2.154 Drawdown-at-Risk (DRAWDAR — ADR-141)

Pulled from `research::get_drawdar`. Computes the Chekhlov et al.
(2005) drawdown quantile: DaR(α) and CDaR(α) at 5% and 1%, the
drawdown analogs of VaR and CVaR. Reports running drawdown series
quantiles directly — orthogonal to CVAR (return tail) and DDHIST
(max/duration statistics). Header gives **drawdar_label** (LOW_DD_RISK
DaR5 < 3% / MODERATE < 8% / ELEVATED < 15% / HIGH ≥ 15% /
INSUFFICIENT_DATA). Body reports bars_used, max_dd, dar_5, cdar_5,
dar_1, cdar_1, dd_events.
Source: ADR-141 DRAWDAR window.

#### 2.155 Volatility Half-Life (VARHALF — ADR-141)

Pulled from `research::get_varhalf`. Fits AR(1) on rolling 20-day
realized vol series: HL = −ln(2)/ln(β). Measures how quickly
volatility shocks dissipate — fast HL ⇒ mean-reverting vol
(short-lived spikes), slow HL ⇒ persistent vol regime changes.
Header gives **varhalf_label** (FAST_REVERT HL < 5d / MODERATE < 15d /
SLOW < 30d / PERSISTENT ≥ 30d / INSUFFICIENT_DATA). Body reports
rv_points, ar1_beta, ar1_alpha, half_life_days, mean_rv, rv_latest.
Source: ADR-141 VARHALF window.

#### 2.156 Return Gini Coefficient (GINI — ADR-141)

Pulled from `research::get_gini`. Computes the Gini coefficient on
|log returns|: (2·Σ(i·|r|_sorted)) / (n·Σ|r|) − (n+1)/n. Measures
concentration of absolute return magnitudes — high Gini ⇒ a few
outsized moves dominate total return; low Gini ⇒ evenly distributed
moves. Orthogonal to KURT (tail weight) and VOLCLUSTER (temporal
clustering). Header gives **gini_label** (LOW_CONCENTRATION Gini
< 0.3 / MODERATE < 0.5 / HIGH < 0.7 / VERY_HIGH ≥ 0.7 /
INSUFFICIENT_DATA). Body reports bars_used, gini, mean_abs_ret,
max_abs_ret, min_abs_ret. Source: ADR-141 GINI window.

#### 2.157 Sample Entropy (SAMPEN — ADR-142)

Pulled from `research::get_sampen`. Computes sample entropy
(Richman & Moorman 2000) with m=2, r=0.2·σ — the self-match-excluded
improvement over ApEn. SampEn = −ln(A/B) where A = template matches
of length m+1 (excluding i==j) and B = template matches of length m
(excluding i==j). More consistent and lower bias than APEN. Header
gives **sampen_label** (REGULAR SampEn < 0.3 / MODERATE < 0.7 /
COMPLEX < 1.2 / HIGHLY_COMPLEX ≥ 1.2 / UNDEFINED if B=0 /
INSUFFICIENT_DATA). Body reports bars_used, embed_dim, tolerance,
a_count, b_count, and sampen. Source: ADR-142 SAMPEN window.

#### 2.158 Permutation Entropy (PERMEN — ADR-142)

Pulled from `research::get_permen`. Computes permutation entropy
(Bandt & Pompe 2002) with m=3 (6 ordinal patterns). Maps each
consecutive m-tuple to its rank permutation and computes Shannon
entropy of the pattern distribution. Captures temporal ordering
structure invisible to ENTROPY (value histogram) and APEN/SAMPEN
(template matching). Normalised H/log₂(m!) ∈ [0,1]. Header gives
**permen_label** (REGULAR H_norm < 0.50 / MODERATE < 0.70 / COMPLEX
< 0.85 / HIGHLY_COMPLEX ≥ 0.85 / INSUFFICIENT_DATA). Body reports
bars_used, embed_dim, patterns_observed, patterns_possible,
permen_raw, permen_normalised. Source: ADR-142 PERMEN window.

#### 2.159 Recovery Factor (RECFACT — ADR-142)

Pulled from `research::get_recfact`. Computes Recovery Factor =
cumulative total return / |max drawdown|. Answers "has the asset
fully recovered from its worst loss?" RF > 1 ⇒ yes. Distinct from
CALMAR/BURKE/STERLING/PAINRATIO which all use annualized return.
Header gives **recfact_label** (DEEP_LOSS RF < −1 / NEGATIVE < 0 /
RECOVERING < 1 / GOOD < 3 / EXCELLENT ≥ 3 / INSUFFICIENT_DATA).
Body reports bars_used, cum_return_pct, max_drawdown_pct,
recovery_factor. Source: ADR-142 RECFACT window.

#### 2.160 KPSS Stationarity Test (KPSS — ADR-142)

Pulled from `research::get_kpss`. Computes the Kwiatkowski-Phillips-
Schmidt-Shin (1992) stationarity test — the formal complement to
ADF. ADF tests H₀: non-stationary; KPSS tests H₀: stationary.
Standard practice reports both. Uses Newey-West long-run variance
with Bartlett kernel and ℓ = floor(4·(n/100)^(2/9)). Critical
values from KPSS (1992) Table 1: 10%=0.347, 5%=0.463, 1%=0.739.
Header gives **kpss_label** (STATIONARY η_μ ≤ 0.347 /
WEAKLY_NONSTATIONARY ≤ 0.463 / NONSTATIONARY > 0.463 /
INSUFFICIENT_DATA). Body reports bars_used, kpss_stat,
lag_truncation, crit_10/5/1, reject_stationary.
Source: ADR-142 KPSS window.

#### 2.161 Spectral Entropy (SPECENT — ADR-142)

Pulled from `research::get_specent`. Computes spectral entropy =
Shannon entropy of normalised power spectral density via DFT on
mean-centred log returns. Measures periodicity in the frequency
domain — low SpecEnt ⇒ dominant frequency components (cyclical
returns); high SpecEnt ⇒ broad spectrum (noise-like). Orthogonal
to ENTROPY (value histogram), APEN/SAMPEN (time-domain templates),
and PERMEN (ordinal patterns). Header gives **specent_label**
(PERIODIC H_norm < 0.50 / MODERATE_PERIODICITY < 0.70 /
BROAD_SPECTRUM < 0.85 / NOISE_LIKE ≥ 0.85 / INSUFFICIENT_DATA).
Body reports bars_used, num_freqs, spectral_entropy_raw,
spectral_entropy_norm, peak_freq_idx, peak_power_share.
Source: ADR-142 SPECENT window.

#### 2.162 Robust Volatility (ROBVOL — ADR-143)

Pulled from `research::get_robvol`. Computes three annualized σ
estimators on trailing daily log returns: classical sample σ, MAD
σ = MAD/0.6745 (Hampel 1974), and IQR σ = IQR/1.349. Also reports
MAD/classical and IQR/classical ratios. When classical σ is inflated
by a small number of extreme returns, the robust ratios drop below
1; values near or above 1 indicate a clean (or sub-Gaussian) tail.
Complements the purely-classical realized-vol family (RV, Parkinson,
EWMA). Header gives **robvol_label** (HEAVY_OUTLIERS avg ratio < 0.60
/ MODERATE_OUTLIERS < 0.80 / CLEAN < 1.10 / LIGHT_TAILS ≥ 1.10 /
INSUFFICIENT_DATA). Body reports bars_used, classical_sigma, mad_sigma,
iqr_sigma, mad_ratio, iqr_ratio. Source: ADR-143 ROBVOL window.

#### 2.163 Rényi Entropy α=2 (RENYIENT — ADR-143)

Pulled from `research::get_renyient`. Computes Rényi entropy of order
α=2 (collision entropy, Rényi 1961) over a Sturges-sized histogram
of trailing log returns: H₂ = −log₂(Σ pᵢ²). The collision probability
Σ pᵢ² directly answers "how likely are two random returns to fall in
the same bin?" — a classical concentration measure. Differs from
Shannon entropy (α=1) by weighting high-probability bins quadratically
rather than logarithmically, emphasising concentration more sharply.
Header gives **renyient_label** (CONCENTRATED H_norm < 0.50 /
MODERATE < 0.70 / DISPERSED < 0.85 / HIGHLY_DISPERSED ≥ 0.85 /
INSUFFICIENT_DATA). Body reports bars_used, num_bins, alpha,
renyi_raw, renyi_normalised, collision_prob.
Source: ADR-143 RENYIENT window.

#### 2.164 Return Quantile Profile (RETQUANT — ADR-143)

Pulled from `research::get_retquant`. Reports the full 9-point
empirical quantile profile of trailing daily log returns: P1, P5,
P10, P25, P50 (median), P75, P90, P95, P99 (all as percents). Also
computes IQR = P75 − P25 and a tail asymmetry ratio (P99+P01)/(P99−P01)
— positive ⇒ right tail extends further; negative ⇒ left tail
dominates. Dense non-parametric snapshot complementing single-point
TAILR/CVAR and the parametric RETSKEW. Header gives **retquant_label**
(LEFT_TAIL_HEAVY asymm < −0.30 / RIGHT_TAIL_HEAVY > 0.30 / WIDE_IQR
IQR > 4% daily / SYMMETRIC). Body reports all nine percentiles, IQR,
and tail_asymmetry. Source: ADR-143 RETQUANT window.

#### 2.165 Multiscale Entropy (MSENT — ADR-143)

Pulled from `research::get_msent`. Computes Sample Entropy (SampEn)
on coarse-grained series at scales τ=1 through τ=5 (Costa, Goldberger,
Peng 2005). Scale τ=1 is the raw series; scale τ=k averages each
block of k consecutive returns. A tolerance r = 0.2·σ of the raw
series is held fixed across scales so SampEn values are comparable.
A decaying MSE curve indicates short-scale noise; sustained or
increasing curves indicate genuine long-range structure. The integral
Σ SampEn(τ) is the Complexity Index. Complements single-scale
APEN/SAMPEN by exposing scale-dependent structure. Header gives
**msent_label** (LONG_RANGE_REGULAR all τ SampEn < 0.3 / DECAYING
τ=5 < 0.7·τ=1 / INCREASING τ=5 > 1.3·τ=1 / SUSTAINED otherwise /
INSUFFICIENT_DATA needs ≥100 returns). Body reports bars_used,
embed_dim, tolerance, max_scale, sampen_scale1..5, msent_complexity_index.
Source: ADR-143 MSENT window.

#### 2.166 EWMA Volatility (EWMAVOL — ADR-143)

Pulled from `research::get_ewmavol`. Computes the RiskMetrics (J.P.
Morgan 1996) exponentially-weighted moving volatility with λ=0.94:
σ²_t = λ·σ²_{t−1} + (1−λ)·r²_t. Effective lookback ≈ 1/(1−λ) ≈ 17
days; recent returns dominate. Reports daily and annualized EWMA σ
alongside the classical sample σ (annualised ×√252) and their ratio
— a regime flag for recent-vs-average volatility. Complements the
equal-weighted RV and the AR(1) half-life from VARHALF. Header gives
**ewmavol_label** (ELEVATED ratio > 1.20 / SUPPRESSED < 0.80 /
NORMAL / INSUFFICIENT_DATA). Body reports bars_used, lambda,
ewma_variance, ewma_sigma_daily, ewma_sigma_annual,
classical_sigma_annual, ewma_to_classical.
Source: ADR-143 EWMAVOL window.

#### 2.167 Kolmogorov-Smirnov Normality Test (KSNORM — ADR-144)

Pulled from `research::get_ksnorm`. Standardises the trailing ≤253
log returns to z = (r−μ̂)/σ̂, sorts them, and computes the
Kolmogorov-Smirnov D statistic = sup |F_emp(z) − Φ(z)|. Emits three
rejection flags vs the classical Kolmogorov critical values
(1.22/√n, 1.36/√n, 1.63/√n) for significance levels 10%/5%/1%. The
sample μ̂ and σ̂ are included so the consumer can reproduce the
standardisation. Header gives **ksnorm_label** (NORMAL fails-to-reject
at 10% / MILD_DEVIATION rejects at 10% but not 5% / MODERATE_DEVIATION
rejects at 5% but not 1% / STRONG_NON_NORMAL rejects at 1% /
INSUFFICIENT_DATA). Body reports bars_used, ks_statistic,
critical_10pct, critical_5pct, critical_1pct, reject booleans, mean,
sigma.
Source: ADR-144 KSNORM window.

#### 2.168 Anderson-Darling Normality Test (ADTEST — ADR-144)

Pulled from `research::get_adtest`. Tail-weighted goodness-of-fit test
for N(μ̂,σ̂²): A² = −n − (1/n)·Σᵢ (2i−1)[ln(Φ(z_i)) +
ln(1−Φ(z_{n+1−i}))], Stephens 1986 small-sample correction
A²_adj = A²·(1 + 0.75/n + 2.25/n²). Compares to the fixed critical
values 0.631/0.752/1.035 at the 10%/5%/1% levels. A p-value
approximation (Stephens 1986 piecewise-exponential) is also emitted
for convenience. Header gives **adtest_label** (same four-way
rejection progression as KSNORM / INSUFFICIENT_DATA). Body reports
bars_used, ad_statistic, ad_adjusted, p_value_approx, critical_10pct,
critical_5pct, critical_1pct, reject booleans.
Source: ADR-144 ADTEST window.

#### 2.169 L-Moments (LMOM — ADR-144)

Pulled from `research::get_lmom`. Computes Hosking's 1990 L-moments
L1..L4 and L-ratios τ3 = L3/L2 (L-skewness) and τ4 = L4/L2 (L-kurtosis)
using unbiased probability-weighted moments b_r =
(1/n)·Σᵢ C(i−1,r)/C(n−1,r)·x_(i) on the trailing log returns. Robust
alternative to classical skew/kurt that stays finite whenever the
mean exists — essential for heavy-tailed financial series. τ3 is
bounded in [−1,1] and τ4 in [−¼, 1] for continuous distributions,
making cross-symbol comparison straightforward. Header gives
**lmom_label** (HEAVY_LEFT τ3 < −0.30 / HEAVY_RIGHT τ3 > 0.30 /
HEAVY_TAILS τ4 > 0.30 / LIGHT_TAILS τ4 < 0.05 / NEAR_SYMMETRIC /
INSUFFICIENT_DATA). Body reports bars_used, l1_mean, l2_scale, l3,
l4, tau3_skew, tau4_kurt.
Source: ADR-144 LMOM window.

#### 2.170 Kyle's Price-Impact λ (KYLELAM — ADR-144)

Pulled from `research::get_kylelam`. Regression coefficient λ =
cov(|Δp|, V) / var(V) on daily absolute close-to-close price change
and share volume — Kyle's (1985) price-impact measure expressing how
many dollars per share of order flow move price. Also reports the
Pearson correlation ρ(|Δp|, V) and R² = ρ² for signal-quality
assessment. Distinct from AMIHUD, which is |r|/$-volume (a scale-free
ratio); KYLELAM is a linear-regression slope on shares with physical
units $-per-share. Header gives **kylelam_label** (HIGH_IMPACT R² >
0.20 / MODERATE_IMPACT R² > 0.05 / LOW_IMPACT / NO_SIGNAL R² < 0.02
/ INSUFFICIENT_DATA). Body reports bars_used, kyle_lambda,
mean_abs_dp, mean_volume, correlation, r_squared.
Source: ADR-144 KYLELAM window.

#### 2.171 Peaks-Over-Threshold (PEAKOVER — ADR-144)

Pulled from `research::get_peakover`. Extreme-value / GPD foundation:
takes |returns|, computes the P95 and P99 thresholds (linear
interpolation, type-7), and reports for each threshold the count of
exceedances, the mean excess |r|−u above the threshold (conditional
on exceeding), and the max excess. The mean-excess / threshold ratio
at P95 is Pickands' GPD-shape-parameter proxy: a high ratio indicates
slowly decaying tails above the threshold. Pickands-Balkema-de Haan
(1974/1975) motivates the threshold-exceedance framing. Header gives
**peakover_label** (EXTREME_TAIL ratio > 0.80 / HEAVY_TAIL > 0.40 /
MODERATE_TAIL > 0.20 / LIGHT_TAIL / INSUFFICIENT_DATA). Body reports
bars_used, threshold_p95, threshold_p99, count_p95, count_p99,
mean_excess_p95, mean_excess_p99, max_excess_p95, max_excess_p99.
Source: ADR-144 PEAKOVER window.

#### 2.172 Higuchi Fractal Dimension (HIGUCHI — ADR-145)

Pulled from `research::get_higuchi`. Higuchi 1988 fractal dimension of
the cumulative log-return walk. For each sub-sampling interval k ∈
1..k_max=10 the normalised path length L(k) is computed; FD is the
negative slope of log L(k) on log k via ordinary least-squares. FD ∈
[1,2] classifies the walk as **SMOOTH** (<1.1, persistent trends
dominate), **PERSISTENT** (<1.4), **RANDOM** (<1.6, Brownian regime)
or **ROUGH** (otherwise, anti-persistent). Header gives
**higuchi_label** + FD + R² (linear-fit quality). Body reports
bars_used, k_max, fractal_dim, r_squared, log_k_count. Complements
Hurst exponent (H=2−FD under Brownian assumptions) as an independent
estimator. Source: ADR-145 HIGUCHI window.

#### 2.173 Pickands Tail-Index (PICKANDS — ADR-145)

Pulled from `research::get_pickands`. Pickands 1975 extreme-value
γ̂ = ln((x_k−x_2k)/(x_2k−x_4k)) / ln 2, valid across all three EV
domains (unlike Hill which assumes Fréchet). Uses k = max(n/16, 5)
ensuring 4k < n. γ̂ maps to tail α = 1/γ̂ (when γ̂ > 0). Header
gives **pickands_label** (FRECHET_HEAVY γ̂>0.5 / FRECHET_MODERATE
γ̂>0.1 / GUMBEL_EXPONENTIAL γ̂>−0.1 / WEIBULL_BOUNDED /
INSUFFICIENT_DATA for degenerate order-statistic triplets). Body
reports bars_used, k_index, gamma_hat, tail_index, x_k, x_2k, x_4k.
Used in concert with HILLTAIL as a Hill/Pickands cross-check — if
the two disagree strongly, the assumed tail model is suspect. Source:
ADR-145 PICKANDS window.

#### 2.174 Kappa-3 Ratio (KAPPA3 — ADR-145)

Pulled from `research::get_kappa3`. Kaplan-Knowles 2004 generalised
Sortino: κ3 = (μ − MAR) / LPM3^(1/3) with MAR=0 and annualisation
(×252 for excess mean, ×252^(1/3) for the cube-root). LPM3 weights
tail losses more heavily than LPM2 — more sensitive to rare extreme
drawdowns than Sortino. Snapshot also carries Sortino (via LPM2^(1/2))
as a reference so the user can regress κ3 vs Sortino to see asymmetry
in the downside. Header gives **kappa3_label** (STRONG κ3>1 /
POSITIVE κ3>0 / NEUTRAL κ3>−0.5 / NEGATIVE / INSUFFICIENT_DATA when
LPM3 is numerically zero). Body reports bars_used, mar, excess_mean,
lpm3, lpm3_root, kappa3, sortino_compare. Source: ADR-145 KAPPA3
window.

#### 2.175 Largest Lyapunov Exponent (LYAPUNOV — ADR-145)

Pulled from `research::get_lyapunov`. Rosenstein et al. 1993 largest
Lyapunov exponent on the embedded time series: m=3 embedding
dimension, τ=1 time delay, Theiler window=10 to exclude temporally
close pairs. For each reference vector, finds its nearest
non-Theiler neighbour and tracks log-distance growth over up to 20
steps. λ_max is the slope of the mean-log-divergence curve via OLS.
λ_max > 0 indicates sensitive dependence on initial conditions
(chaotic); λ_max ≈ 0 indicates periodic/quasi-periodic; λ_max < 0
indicates stable convergence. Header gives **lyapunov_label**
(CHAOTIC λ>0.10 / WEAKLY_CHAOTIC λ>0.02 / PERIODIC λ>−0.02 / STABLE
/ INSUFFICIENT_DATA for degenerate embeddings or < 5 regression
points). Body reports bars_used, embed_dim, time_delay, lambda_max,
r_squared, steps_used. Complements RUNS (randomness) and PACF
(linear dependence) on the nonlinear-dynamics axis neither touches.
Source: ADR-145 LYAPUNOV window.

#### 2.176 Spearman Rank Autocorrelation (RANKAC — ADR-145)

Pulled from `research::get_rankac`. Nonparametric Pearson ACF:
rank-transform the return series using average ranks for ties
(Spearman convention), then compute Pearson ρ at lags 1, 5, 10.
Robust under fat tails and invariant under monotone transforms —
useful for heavy-tailed assets where Pearson ACF over-weights tail
observations. Header gives **rankac_label** (STRONG_DEPENDENCE
max|ρ|>0.30 / MODERATE_DEPENDENCE >0.15 / WEAK_DEPENDENCE >0.05 /
INDEPENDENT / INSUFFICIENT_DATA for n<30). Body reports bars_used,
rho_lag1, rho_lag5, rho_lag10, mean_abs_rho, max_abs_rho. Direct
robust counterpart to PACF; disagreement between the two suggests
heavy-tail-driven spurious Pearson ACF signal. Source: ADR-145
RANKAC window.

#### 2.177 Barndorff-Nielsen-Shephard Jump Test (BNSJUMP — ADR-146)

Pulled from `research::get_bnsjump`. BNS 2006 Z-statistic for the null
hypothesis of no jumps vs the alternative of jump-augmented diffusion:
z = (RV − BV) / sqrt(θ · Σr⁴) where RV = Σr_i² is the realised
variance, BV = (π/2) · Σ|r_{i-1}·r_i| is the bipower variation (which
converges to the diffusive component only under the jump alternative),
and θ = π²/4 + π − 5 standardises under the null. Header gives
**bnsjump_label** (STRONG_JUMP z>3.09 / MODERATE_JUMP >2.33 /
WEAK_JUMP >1.65 / NO_JUMP / INSUFFICIENT_DATA). Body reports
bars_used, realized_variance, bipower_variance, jump_ratio
(RV−BV)/RV, jump_z_stat, p_value (approx 1−Φ(|z|)). Formal
hypothesis-test version of Round 30's raw BIPOWER surface. Source:
ADR-146 BNSJUMP window.

#### 2.178 Phillips-Perron Unit-Root Test (PPROOT — ADR-146)

Pulled from `research::get_pproot`. Third stationarity surface
alongside ADF (ADR-126) and KPSS (ADR-144). Phillips & Perron (1988)
nonparametric: estimate ρ from y_t = ρ·y_{t-1} + ε_t via OLS, then
apply Newey-West corrections using a Bartlett kernel with lag
truncation q = floor(4·(n/100)^0.25) per Schwert 1989. Header gives
**pproot_label** (STATIONARY_STRONG Z(t)<−3.43 / STATIONARY_WEAK
<−2.86 / BORDERLINE <−2.57 / UNIT_ROOT / INSUFFICIENT_DATA). Body
reports bars_used, rho_hat, t_rho (raw), z_rho (PP Z(ρ)), z_t (PP
Z(t)), lag_truncation. Robust to conditional heteroscedasticity —
three-way ADF/KPSS/PP agreement is a strong stationarity call.
Source: ADR-146 PPROOT window.

#### 2.179 Multifractal DFA (MFDFA — ADR-146)

Pulled from `research::get_mfdfa`. Kantelhardt 2002 generalisation of
DFA: at each of 7 scales s ∈ {8, 12, 16, 24, 32, 48, 64} (bounded by
n/4), split the cumulative return walk into non-overlapping windows,
linearly detrend each window to get F²(s,v), then aggregate via the
q-order moment: F_q(s) = [(1/N_s) Σ F²(s,v)^(q/2)]^(1/q) for q≠0,
F_0(s) = exp[(1/2N_s) Σ ln F²(s,v)]. Fit h(q) = slope of ln F_q(s) vs
ln s. Header gives **mfdfa_label** (STRONG_MULTIFRACTAL Δh>0.30 /
MODERATE_MULTIFRACTAL >0.15 / WEAK_MULTIFRACTAL >0.05 / MONOFRACTAL /
INSUFFICIENT_DATA for n<120). Body reports bars_used, h_q_neg2,
h_q_zero, h_q_pos2, delta_h, scales_used. First multifractal
spectrum surface — complements monofractal HURST/DFA/HIGUCHI.
Source: ADR-146 MFDFA window.

#### 2.180 Hill-Tail KS Goodness-of-Fit (HILLKS — ADR-146)

Pulled from `research::get_hillks`. KS test between the empirical
tail distribution and the fitted Pareto model implicit in the Hill
estimator. Take the top k = floor(n·0.10) absolute log-returns, fit
α̂ via the standard Hill formula 1/α̂ = (1/k) Σ ln(x_i/x_{k+1}), then
compute D = sup|F_n(y) − (1 − y^{−α̂})| over the tail sample where
y = x/x_{k+1}. Critical value is 1.36/√k at 5%. Header gives
**hillks_label** (GOOD_FIT D<0.5·crit / ACCEPTABLE_FIT <0.9·crit /
POOR_FIT <1.3·crit / REJECT / INSUFFICIENT_DATA for n<50). Body
reports bars_used, k_order, alpha_hat, ks_statistic,
ks_critical_5pct. Catches cases where HILLTAIL's α̂ is
quantitative nonsense because the tail shape doesn't actually fit a
Pareto. Source: ADR-146 HILLKS window.

#### 2.181 True Strength Index (TSI — ADR-146)

Pulled from `research::get_tsi`. Blau 1991 double-smoothed momentum
oscillator: TSI = 100 · EMA_short(EMA_long(ΔP)) /
EMA_short(EMA_long(|ΔP|)) with classical 25/13 periods. Zero-line
crossovers signal momentum flips; TSI−signal (where signal is a
second 13-period EMA of TSI itself) triggers entries on
momentum-of-momentum. Header gives **tsi_label** (STRONG_BULL
TSI>25 / BULL >0 / NEUTRAL |TSI|<5 / BEAR >−25 / STRONG_BEAR /
INSUFFICIENT_DATA for n<60). Body reports bars_used, ema_long,
ema_short, tsi_value, signal_value, tsi_minus_signal. Cleaner
zero-line behaviour than RSI; less noisy than MACD; distinct from
CCI by using dual-EMA smoothing rather than mean deviation. Source:
ADR-146 TSI window.

#### 2.182 GARCH(1,1) Fit (GARCH11 — ADR-147)

Pulled from `research::get_garch11`. Bollerslev 1986 conditional
variance model σ²_t = ω + α·r²_{t-1} + β·σ²_{t-1} fit by
coordinate-descent grid MLE over (α, β) ∈ [0, 0.4] × [0.4, 0.99]
with ω implied by the unconditional-variance constraint ω =
var·(1−α−β). Header gives **garch11_label** (NEAR_INTEGRATED
α+β≥0.99 / HIGH_PERSISTENCE >0.95 / MODERATE_PERSISTENCE >0.85 /
LOW_PERSISTENCE / INSUFFICIENT_DATA for n<60). Body reports
bars_used, ω, α, β, persistence (α+β), unconditional variance,
half-life in bars (ln 0.5 / ln(α+β)), and log-likelihood. First
parametric volatility-persistence model in the terminal; EWMAVOL's
single-λ decay is RiskMetrics-style, GARCH11 is the industry
standard 2-parameter decomposition. Source: ADR-147 GARCH11 window.

#### 2.183 Sup-ADF Bubble Test (SADF — ADR-147)

Pulled from `research::get_sadf`. Phillips-Wu-Yu 2011 explosive-root
statistic — expanding-window ADF t over r0 = floor((0.01 +
1.8/√n)·n) forward, sup at the end. Complements the three
stationarity tests (ADF/KPSS/PPROOT) by asking the asymmetric
question: "is there an explosive sub-window in the recent tail?".
Header gives **sadf_label** (EXPLOSIVE_CONFIRMED SADF>1.5·crit /
EXPLOSIVE_LIKELY >crit / BORDERLINE >0.8·crit / STABLE /
INSUFFICIENT_DATA for n<60). Body reports bars_used, min_window r0,
full-sample ADF-t, sup-ADF statistic, argmax end index, tabulated
5% critical value (interpolated in n), and reject-null boolean.
First bubble / explosive-root detector in the terminal. Source:
ADR-147 SADF window.

#### 2.184 Correlation Dimension (CORDIM — ADR-147)

Pulled from `research::get_cordim`. Grassberger-Procaccia 1983
correlation dimension D2 at embedding m=3, fit to 10 log-spaced
radii spanning 0.1× to ~1.0× the standardised-return range. D2 =
d log C(ε) / d log ε where C(ε) is the fraction of m-vector pairs
within ε. Header gives **cordim_label** (LOW_DIM D2<1.5 /
MODERATE_DIM <2.5 / HIGH_DIM <3.5 / STOCHASTIC otherwise /
INSUFFICIENT_DATA for n<60). Body reports bars_used, embed_dim,
radii fitted, D2, and fit R². Distinct from Hurst/DFA/Higuchi —
those assume self-similar scaling, D2 quantifies effective
dimensionality of the embedded dynamics. Low D2 = close to a
low-dimensional attractor; high D2 = near-stochastic. Source:
ADR-147 CORDIM window.

#### 2.185 Rolling Skewness Spectrum (SKSPEC — ADR-147)

Pulled from `research::get_skspec`. Rolling 30-bar window skewness,
then mean/std/min/max/range of the resulting skew series. Answers
"is the return skew *stable* over time, or does it flip
sign?" — critical for strategies whose P&L depends on skew
persistence (e.g. put-selling). Header gives **skspec_label**
(STABLE_POSITIVE |mean|>2·std & mean>0 / STABLE_NEGATIVE
|mean|>2·std & mean<0 / DRIFTING |mean|>std / UNSTABLE otherwise /
INSUFFICIENT_DATA for n<60). Body reports bars_used, window_size,
mean_skew, std_skew, min_skew, max_skew, range_skew. Complements
RETQUANT (ADR-135) which ships full-window skew — SKSPEC says
whether that number is reliable. Source: ADR-147 SKSPEC window.

#### 2.186 Auto Mutual Information (AUTOMI — ADR-147)

Pulled from `research::get_automi`. Information-theoretic ACF:
MI(k) = I(X_t; X_{t-k}) estimated via k=8 equiprobable histogram
bins at lags 1, 5, 10 — plus the marginal entropy H(X) and the
normalised MI(1)/H(X) ∈ [0,1]. Catches *any* statistical
dependence (including nonlinear) — signature of volatility
clustering, which contributes ~zero to linear ACF of returns but
dominates MI of |returns|. Header gives **automi_label** (STRONG
MI(1)/H(X)>0.20 / MODERATE >0.10 / WEAK >0.03 / INDEPENDENT
otherwise / INSUFFICIENT_DATA for n<50). Body reports bars_used,
num_bins, MI(1/5/10), H(X), and normalised MI(1)/H(X). Source:
ADR-147 AUTOMI window.

#### 2.187 Durbin-Watson Autocorrelation (DURBINWATSON — ADR-149)

Pulled from `research::get_durbinwatson`. Classic Durbin-Watson
d-statistic on log-returns: d = Σ(Δe)² / Σe² with e = r − mean(r).
d∈[0,4], d≈2 indicates no first-order AR(1) correlation, d<1 strong
positive, d>3 strong negative; implied ρ̂ ≈ 1 − d/2. Header gives
**dw_label** (STRONG_POS d<1 / WEAK_POS <1.5 / NO_AUTOCORR 1.5–2.5 /
WEAK_NEG <3.0 / STRONG_NEG otherwise / INSUFFICIENT_DATA for n<30).
Body reports bars_used, dw_stat, rho_estimate. Complements LJUNGB
(block-lag) and AUTOMI (information-theoretic) by surfacing the
single-lag linear diagnostic from the classical regression-printout
tradition. Source: ADR-149 DURBINWATSON window.

#### 2.188 BDS iid Test (BDSTEST — ADR-149)

Pulled from `research::get_bdstest`. Brock-Dechert-Scheinkman (1996)
test of the iid null at embedding dimension m=2 with ε=0.7×σ. Reports
the asymptotically-standard-normal BDS statistic computed from the
correlation integrals C_1(ε) and C_m(ε); two-sided p-value; and a
reject_null flag at α=0.05. Header gives **bds_label**
(IID_CONFIRMED p≥0.05 / WEAK_DEPENDENCE |BDS|<4 / STRONG_DEPENDENCE
otherwise / INSUFFICIENT_DATA for n<100). Body reports bars_used,
embed_dim, epsilon_mult, bds_stat, p_value_two_sided, reject_null.
Variance approximation is a simplified upper bound (see ADR-149); a
rejection is robust evidence of nonlinear dependence but marginal
p-values should be cross-checked against a dedicated package.
Source: ADR-149 BDSTEST window.

#### 2.189 Breusch-Pagan Heteroskedasticity (BREUSCHPAGAN — ADR-149)

Pulled from `research::get_breuschpagan`. Breusch-Pagan (1979) LM
test with bar index as the sole regressor on squared residuals of
the demeaned log-returns. Statistic LM = n×R² compared to χ²(1);
critical_95 = 3.841. Header gives **bp_label** (HOMOSKEDASTIC
LM≤3.841 / MILD_HETERO <10.83 / STRONG_HETERO otherwise /
INSUFFICIENT_DATA for n<30). Body reports bars_used, lm_stat,
r_squared, df, critical_95, reject_null. Complements ARCHLM
(ADR-139) which tests autoregressive conditional heteroskedasticity;
BREUSCHPAGAN catches monotonic trends in variance that ARCHLM misses.
Source: ADR-149 BREUSCHPAGAN window.

#### 2.190 Bartels Turning-Points Test (TURNPTS — ADR-149)

Pulled from `research::get_turnpts`. Non-parametric Bartels /
turning-points test on log-returns: counts strict local extrema
(b>a ∧ b>c or b<a ∧ b<c), compares observed against expected
2(n−2)/3 under the iid null, variance (16n−29)/90, z-statistic, and
two-sided p-value. Header gives **turnpts_label** (RANDOM_IID
p≥0.05 / OVER_TURNING z>0 / UNDER_TURNING z<0 / INSUFFICIENT_DATA
for n<10). Body reports bars_used, observed_turnpts,
expected_turnpts, variance_turnpts, z_stat, p_value_two_sided,
reject_null. Orthogonal to RUNSTEST (sign-relative-to-median) —
catches different iid failure modes. Source: ADR-149 TURNPTS window.

#### 2.191 Direct-DFT Periodogram (PERIODOGRAM — ADR-149)

Pulled from `research::get_periodogram`. Schuster-style raw
periodogram computed by direct DFT over k=1..min(n/2, 256) on the
mean-centered log-returns. Reports the dominant frequency, its
period in bars (1/f), its power, total spectral power, and the
dominant-to-total ratio. Header gives **periodogram_label**
(STRONG_CYCLE ratio>0.25 / MODERATE_CYCLE >0.12 / WEAK_CYCLE >0.05 /
NO_CYCLE otherwise / INSUFFICIENT_DATA for n<64). Body reports
bars_used, n_freqs, dominant_freq, dominant_period_bars,
dominant_power, total_power, dominant_power_ratio. First
frequency-domain surface in the packet — complements the time-domain
DFA/MFDFA/AUTOMI family. Leakage is uncorrected (no windowing); for
exact peak positioning cross-check with a multitaper estimator.
Source: ADR-149 PERIODOGRAM window.

#### 2.192 McLeod-Li Squared-Returns Portmanteau (MCLEODLI — ADR-150)

Pulled from `research::get_mcleodli`. Portmanteau Ljung-Box-style test
applied to *squared* log-returns (not levels) to detect ARCH effects.
Q = n(n+2) Σ_k=1..h ρ̂²(k)/(n−k) where ρ̂(k) is the sample autocorrelation
of r_t² at lag k. Compared against χ²(h), h = max(5, min(10, n/5)). Header
gives **mcleodli_label** (NO_ARCH p≥0.05 / MILD_ARCH Q<2·critical /
STRONG_ARCH otherwise / INSUFFICIENT_DATA for n<30). Body reports
bars_used, lag_h, q_stat, df, critical_95, p_value, reject_null.
Complements ARCHLM (LM regression) and LJUNGB (portmanteau on levels).
Source: ADR-150 MCLEODLI window.

#### 2.193 Ornstein-Uhlenbeck Mean-Reversion Fit (OUFIT — ADR-150)

Pulled from `research::get_oufit`. Fits an OLS AR(1) on log-prices
x_{t+1} = a + b·x_t + ε and derives the continuous-time OU
parametrization θ = −ln(b), μ = a/(1−b), σ = residual sd, half-life =
ln(2)/θ. Header gives **oufit_label** (TRENDING θ≤0 / SLOW_REVERT
HL > n/3 / MODERATE_REVERT HL > n/10 / FAST_REVERT otherwise /
INSUFFICIENT_DATA for n<30). Body reports bars_used, theta, mu, sigma,
half_life_bars (∞ when θ≤0), residual_sd, r_squared. First explicit
SDE-parametrization surface; complements MRHL's implied-half-life view.
Source: ADR-150 OUFIT window.

#### 2.194 Geweke-Porter-Hudak Long-Memory d̂ (GPH — ADR-150)

Pulled from `research::get_gph`. Semiparametric log-periodogram
regression for the fractional integration order d. Computes I(λ_j) on
Fourier frequencies j=1..m where m = floor(n^0.5), then regresses
ln I(λ_j) on −2 ln|2 sin(λ_j/2)| and extracts d = −slope/2. Reports the
π²/(24m)-stderr, the t-statistic for H0: d=0, and the two-sided
p-value. Header gives **gph_label** (ANTIPERSISTENT d<−0.1 /
SHORT_MEMORY |d|≤0.1 / LONG_MEMORY 0.1<d<0.5 / NONSTATIONARY d≥0.5 /
INSUFFICIENT_DATA for n<64). Body reports bars_used, m_freqs,
d_estimate, d_stderr, t_stat, p_value_two_sided. Classical
semiparametric complement to HURST/DFA/HIGUCHI/MFDFA's fractal-dimension
angles. Source: ADR-150 GPH window.

#### 2.195 Burg Maximum-Entropy AR Spectrum (BURGSPEC — ADR-150)

Pulled from `research::get_burgspec`. Parametric spectral estimator:
fits an AR(p) model via the Burg lattice recursion (Marple 1987, §6.6),
p = min(20, n/4), then evaluates the resulting spectral density on a
256-point grid over (0, 0.5] and reports the dominant peak. Header
gives **burgspec_label** (NO_AR_CYCLE peak/mean≤2 / WEAK_AR_CYCLE ≤4 /
MODERATE_AR_CYCLE ≤8 / STRONG_AR_CYCLE otherwise / INSUFFICIENT_DATA
for n<32). Body reports bars_used, ar_order, dominant_freq,
dominant_period_bars, peak_power, mean_power, peak_to_mean_ratio.
Parametric complement to the non-parametric PERIODOGRAM. Better peak
resolution on short series at the cost of AR-order sensitivity.
Source: ADR-150 BURGSPEC window.

#### 2.196 Kendall's Tau Lag-1 Rank Autocorrelation (KENDALLTAU — ADR-150)

Pulled from `research::get_kendalltau`. Non-parametric rank
autocorrelation on log-returns at lag-1. Pairs (r_i, r_{i+1}) form the
working series; τ = (C − D) / [m(m−1)/2] where C and D count
concordant vs discordant index pairs. Asymptotic z-statistic
τ/sqrt(2(2m+5)/(9m(m−1))). Header gives **kendalltau_label**
(STRONG_POS τ>0.1 / WEAK_POS >0.03 / NO_RANK_AUTO / WEAK_NEG <−0.03 /
STRONG_NEG <−0.1 / INSUFFICIENT_DATA for n<30). Body reports
bars_used, pair_count, concordant, discordant, tau, z_stat,
p_value_two_sided. Rank-based complement to DURBINWATSON's linear AR(1)
and RANKAC's Spearman lag. Source: ADR-150 KENDALLTAU window.

#### 2.197 Composite Short-Squeeze Score (SQUEEZE — ADR-151)

Pulled from `research::get_squeeze`. Composite short-squeeze probability
score combining five orthogonal axes: short % of float (saturates at 40%),
days-to-cover (saturates at 10 days, SI/20d-avg-volume), 20-day price
momentum (saturates at 30% move), relative volume vs 20d average (saturates
at 3× RV), and IV-rank percentile (0..100). Each axis is normalised to a
0..100 score via a saturating linear curve; the composite is a weighted
mean with **1.5× weight on short-float and days-to-cover** (the mechanical
axes) and **1.0× on momentum / relvol / IV-rank** (the trigger axes), then
re-normalised to 0..100 by the active weight sum. Header gives
**squeeze_label** (NO_SQUEEZE <20 / WATCH <40 / ELEVATED <60 / STRONG <80 /
EXTREME ≥80 / INSUFFICIENT_DATA when <3 axes have data). Body reports
bars_used, the five raw inputs, the five per-axis scores, composite_score,
and inputs_present (0..5). Source: ADR-151 SQUEEZE window.

#### 2.198 Cross-Symbol Short-Squeeze Rank (SQUEEZERANK — ADR-151)

Pulled from `research::get_squeezerank`. Cross-symbol percentile rank of
SQUEEZE composite scores across every symbol with a populated SQUEEZE row.
Scanned by the SQUEEZE watchlist driver. Header gives **squeezerank_label**
(TOP_1PCT / TOP_5PCT / TOP_10PCT / ABOVE_MEDIAN / BELOW_MEDIAN /
INSUFFICIENT_DATA for peer_count<10). Body reports composite_score (mirror
of SQUEEZE), peer_count, rank (1 = highest), percentile. Complements SQUEEZE
with peer-group context — a 60-composite score means very different things
when the sector tape is quiet versus running hot. Source: ADR-151 SQUEEZERANK.

#### 2.199 Bollinger Band Width Squeeze (BBSQUEEZE — ADR-151)

Pulled from `research::get_bbsqueeze`. Classical Bollinger-band-width
percentile-rank squeeze detector: computes BB-width = (upper − lower) / mid
over period=20 with 2σ bands on the trailing 120-bar window, then ranks the
current bar's width against its own 120-bar distribution. Header gives
**bbsqueeze_label** (TIGHT_SQUEEZE percentile ≤10 / MODERATE_SQUEEZE ≤25 /
NORMAL ≤75 / EXPANSION >75 / INSUFFICIENT_DATA for n<140). Body reports
bars_used, period, bb_width_current, bb_width_min_120, bb_width_max_120,
bb_width_percentile, upper/lower/mid band values, last_close. The
volatility-contraction complement to the position-price SQUEEZE composite —
BBSQUEEZE flags *range compression* regardless of short interest. Source:
ADR-151 BBSQUEEZE window.

#### 2.200 Donchian Channel Breakout (DONCHIAN — ADR-151)

Pulled from `research::get_donchian`. 20-bar Donchian-channel breakout
detector. upper_channel = max(high_{t-19..t-1}), lower_channel =
min(low_{t-19..t-1}); breakout flags are set when the current close equals
or exceeds the **prior** channel (excluding self-reference). Header gives
**donchian_label** (BREAKOUT_UP / APPROACH_UP position ≥80 / NEUTRAL /
APPROACH_DOWN ≤20 / BREAKOUT_DOWN / INSUFFICIENT_DATA for n<21). Body
reports bars_used, period, upper_channel, lower_channel, mid_channel,
last_close, channel_position_pct (0..100), breakout_upper/lower flags.
Classical trend-following breakout surface (Turtle Traders). Source:
ADR-151 DONCHIAN window.

#### 2.201 Kaufman Adaptive Moving Average (KAMA — ADR-151)

Pulled from `research::get_kama`. Kaufman Adaptive Moving Average with its
Efficiency Ratio. ER = |close_t − close_{t-n}| / Σ|close_i − close_{i-1}|
at n=10 — the ratio of net-directional move to path-length. Smoothing
constant SC = [ER·(2/(fast+1)) + (1−ER)·(2/(slow+1))]² with fast=2,
slow=30. KAMA is the recursive filter KAMA_t = KAMA_{t-1} + SC·(close −
KAMA_{t-1}), seeded from the n-bar SMA. Header gives **kama_label**
(STRONG_TREND ER>0.5 / MODERATE_TREND >0.3 / WEAK_TREND >0.15 / CHOPPY /
INSUFFICIENT_DATA for n<16). Body reports bars_used, period (10),
efficiency_ratio (0..1), kama_value, last_close, kama_slope_pct over 5
bars. Trend-quality complement to DONCHIAN's breakout detector — DONCHIAN
answers *did we break*, KAMA answers *is the move clean enough to trade*.
Source: ADR-151 KAMA window.

#### 2.202 Ichimoku Kinkō Hyō Cloud (ICHIMOKU — ADR-152)

Pulled from `research::get_ichimoku`. Canonical Japanese one-glance
equilibrium chart: Tenkan-sen(9) = (maxH9 + minL9)/2, Kijun-sen(26) =
(maxH26 + minL26)/2, Senkou Span A = (Tenkan + Kijun)/2, Senkou Span
B(52) = (maxH52 + minL52)/2, Chikou Span = close shifted back 26.
Header gives **ichimoku_label** (STRONG_BULL close > Senkou A & B AND
Tenkan > Kijun / BULL close > cloud / NEUTRAL / BEAR close < cloud /
STRONG_BEAR close < cloud AND Tenkan < Kijun / INSUFFICIENT_DATA for
n<78). Body reports bars_used, all five line values, cloud_top (max
of Senkou A/B), cloud_bottom (min), last_close. Cloud + T/K cross
summarise trend + support/resistance + momentum in a single pass.
Source: ADR-152 ICHIMOKU window.

#### 2.203 SuperTrend ATR-Channel Overlay (SUPERTREND — ADR-152)

Pulled from `research::get_supertrend`. Wilder-ATR trailing-stop band
with strict flip recursion. Period=10, multiplier=3; upper/lower
half-bands = hl2 ± m·ATR; the active band only tightens in the
trend direction until the close crosses it, at which point the band
flips and the sign of `direction` inverts. Header gives
**supertrend_label** (STRONG_BULL / BULL / NEUTRAL / BEAR /
STRONG_BEAR / INSUFFICIENT_DATA for n<11). Body reports bars_used,
period, multiplier, atr, supertrend_value, direction (+1 long / −1
short), last_close, distance_pct. Regime-aware complement to
DONCHIAN's pure N-bar envelope breakout. Source: ADR-152 SUPERTREND.

#### 2.204 Keltner Channel + TTM-Squeeze (KELTNER — ADR-152)

Pulled from `research::get_keltner`. Keltner Channel (EMA-20 midline
± 2·ATR-10) with an inline Bollinger(20, 2σ) computed for the
TTM-Squeeze detection. Header gives **keltner_label** (STRONG_BULL
close > upper / BULL / NEUTRAL / BEAR / STRONG_BEAR close < lower /
INSUFFICIENT_DATA for n<21). Body reports bars_used, period (20),
atr_period (10), multiplier (2), upper/mid/lower KC bands, bb_upper,
bb_lower (inline), last_close, **ttm_squeeze** boolean (true when
BB_upper ≤ KC_upper AND BB_lower ≥ KC_lower → volatility
compression / breakout precursor, John Carter 2005). TTM-squeeze
pairs KELTNER + BBSQUEEZE (ADR-151) for the canonical John Carter
construct. Source: ADR-152 KELTNER window.

#### 2.205 Ehlers Fisher Transform (FISHER — ADR-152)

Pulled from `research::get_fisher`. Ehlers (2002) price-distribution
transform 0.5·ln((1+x)/(1−x)) on hl2 midline rescaled over a 10-bar
window to [−0.999, 0.999] with 0.66/0.67 smoothing weights and 0.5
prior feedback. Sharper peaks than raw returns for turning-point
detection. Header gives **fisher_label** (PEAK_HIGH fisher > 2 /
BULL > 0.5 / NEUTRAL / BEAR < −0.5 / PEAK_LOW < −2 /
INSUFFICIENT_DATA for n<11). Body reports bars_used, period (10),
fisher_value, trigger_value (prior fisher), fisher−trigger delta,
last_close. PEAK labels flag saturated regions about to revert;
complementary oscillator to TSI. Source: ADR-152 FISHER window.

#### 2.206 Aroon Oscillator (AROON — ADR-152)

Pulled from `research::get_aroon`. Chande (1995) time-since-extreme
oscillator. Over a 25-bar rolling window we locate bars_since_highest
and bars_since_lowest; Aroon_Up = 100·(25 − bsh)/25, Aroon_Down =
100·(25 − bsl)/25, Oscillator = Up − Down ∈ [−100, +100]. Header
gives **aroon_label** (STRONG_UP osc > 50 / WEAK_UP > 25 /
CONSOLIDATION / WEAK_DOWN < −25 / STRONG_DOWN < −50 /
INSUFFICIENT_DATA for n<26). Body reports bars_used, period (25),
aroon_up, aroon_down, aroon_oscillator, bars_since_high,
bars_since_low, last_close. Distinct from ADX/CHOP (trend-strength)
— Aroon fires the moment a new 25-bar extreme prints, so flags fresh
trends earlier than strength-based indicators. Source: ADR-152 AROON
window.

#### 2.207 Wilder's Average Directional Index (ADX — ADR-153)

Pulled from `research::get_adx`. Classical Wilder (1978) directional-movement
system at period=14. +DM = max(H−H_prev, 0), −DM = max(L_prev−L, 0),
winner of each bar smoothed by Wilder's averaging; +DI = 100·smoothed(+DM)/ATR,
−DI = 100·smoothed(−DM)/ATR; DX = 100·|+DI − −DI|/(+DI + −DI); ADX is
Wilder-smoothed DX. Header gives **adx_label** (STRONG_TREND adx≥40 / TREND
≥25 / WEAK_TREND ≥15 / NO_TREND / INSUFFICIENT_DATA for n<30). Body reports
bars_used, period (14), plus_di, minus_di, adx, dx, atr, last_close.
Complements AROON (ADR-152) which measures *time-since-extreme* — ADX
measures *strength regardless of time*. Source: ADR-153 ADX window.

#### 2.208 Lambert Commodity Channel Index (CCI — ADR-153)

Pulled from `research::get_cci`. Lambert (1980) mean-deviation-normalised
momentum oscillator at period=20. TP=(H+L+C)/3, MAD = mean|TP − SMA(TP,20)|,
CCI = (TP − SMA) / (0.015·MAD) — the 0.015 constant chosen by Lambert so
~70–80% of values fall in [−100, +100]. Header gives **cci_label**
(OVERBOUGHT >100 / BULL >0 / NEUTRAL / BEAR <0 / OVERSOLD <−100 /
INSUFFICIENT_DATA for n<21). Body reports bars_used, period (20),
typical_price, tp_sma, mean_abs_dev, cci_value, last_close. Distinct
from RSI: mean-deviation normalisation rather than gain/loss ratio, so
one-sided slow grinds register different extremes. Source: ADR-153 CCI window.

#### 2.209 Chaikin Money Flow (CMF — ADR-153)

Pulled from `research::get_cmf`. Chaikin (1980s) volume-weighted
accumulation/distribution oscillator at period=20. MFV =
((C−L) − (H−C))/(H−L) × volume (the "money flow volume" per bar);
CMF = Σ MFV / Σ volume over 20 bars ∈ [−1, +1]. Header gives
**cmf_label** (STRONG_ACCUM >0.25 / ACCUM >0.05 / NEUTRAL / DIST <−0.05
/ STRONG_DIST <−0.25 / INSUFFICIENT_DATA for n<21). Body reports
bars_used, period (20), cmf_value, money_flow_volume_sum, volume_sum,
last_close. First volume-weighted accumulation-line surface we ship;
flat doji bars (H==L) are epsilon-guarded so they emit MFV=0 rather than
NaN. Source: ADR-153 CMF window.

#### 2.210 Money Flow Index (MFI — ADR-153)

Pulled from `research::get_mfi`. Quong & Soudack's (1989)
volume-weighted RSI at period=14. Typical-price × volume = "raw money
flow" per bar; bars with TP rising count toward positive flow, falling
toward negative; ratio = Σpos / Σneg; MFI = 100 − 100/(1+ratio). Output
∈ [0, 100]. Header gives **mfi_label** (OVERBOUGHT >80 / BULL >50 /
NEUTRAL / BEAR <50 / OVERSOLD <20 / INSUFFICIENT_DATA for n<15).
Body reports bars_used, period (14), mfi_value, positive_mf_sum,
negative_mf_sum, money_flow_ratio, last_close. Volume-weighted
complement to RSI — bars with heavy volume count more toward the
oscillator. Source: ADR-153 MFI window.

#### 2.211 Wilder Parabolic Stop-And-Reverse (PSAR — ADR-153)

Pulled from `research::get_psar`. Wilder's (1978) accelerating
trailing-stop. Initial acceleration factor (AF) 0.02, increment 0.02
each time a new extreme point (EP) is made, capped at 0.20. SAR_next =
SAR + AF·(EP − SAR); flips when price crosses SAR, with the new SAR
clamped to the prior-two-bar low (long→short) or high (short→long).
Header gives **psar_label** (STRONG_UP / UP / FLAT / DOWN / STRONG_DOWN
/ INSUFFICIENT_DATA for n<6). Body reports bars_used, af_start,
af_step, af_max, sar_value, extreme_point, acceleration_factor,
trend_is_up, bars_in_trend, distance_pct, last_close. Complements
SUPERTREND (ADR-152): PSAR accelerates (AF grows each new EP),
SuperTrend is ATR-proportional and does not accelerate — so PSAR fires
trailing-stop exits earlier in mature trends. Source: ADR-153 PSAR window.

#### 2.212 Botes & Siepman Vortex Indicator (VORTEX — ADR-154)

Pulled from `research::get_vortex`. Botes & Siepman (2009) directional-movement
alternative to ADX at period=14. VM+ = |H_t − L_{t−1}|, VM− = |L_t − H_{t−1}|,
VI+ = ΣVM+ / ΣTR, VI− = ΣVM− / ΣTR. Header gives **vortex_label**
(BULL_CROSS VI+>VI− with VI+>1 / BULL VI+>VI− / NEUTRAL / BEAR VI−>VI+ /
BEAR_CROSS VI−>VI+ with VI−>1 / INSUFFICIENT_DATA for n<16). Body reports
bars_used, period (14), vi_plus, vi_minus, vi_diff, sum_tr, sum_vm_plus,
sum_vm_minus, last_close. Complements ADX (ADR-153): ADX is Wilder-smoothed
and lagged, VORTEX is unsmoothed and catches direction changes earlier.
Source: ADR-154 VORTEX window.

#### 2.213 Bill Dreiss Choppiness Index (CHOP — ADR-154)

Pulled from `research::get_chop`. Dreiss (1980s) bounded 0–100 trend-vs-range
scalar at period=14. CHOP = 100 · log10(ΣTR / (maxH − minL)) / log10(N).
Values >61.8 indicate choppy/ranging, <38.2 indicate trending (Fibonacci
complements). Header gives **chop_label** (CHOP >61.8 / RANGING >50 /
NEUTRAL / TRANSITIONAL <50 / TRENDING <38.2 / INSUFFICIENT_DATA for n<15).
Body reports bars_used, period (14), chop_value, sum_tr, range_high,
range_low, range_span, last_close. Distinct from ADX: ADX measures *trend
strength*, CHOP measures *range efficiency* and is bounded by construction.
Flat-tape (maxH==minL) bars are guarded and emit INSUFFICIENT_DATA rather
than NaN. Source: ADR-154 CHOP window.

#### 2.214 Granville On-Balance Volume (OBV — ADR-154)

Pulled from `research::get_obv`. Granville (*New Key to Stock Market
Profits*, 1963) cumulative volume indicator: OBV_t = OBV_{t−1} + sign(ΔClose)·
Volume_t. Since value depends on history, we pair it with a 20-bar
linear-regression slope normalised against the 20-bar OBV range to emit
a label. Header gives **obv_label** (STRONG_UP / UP / NEUTRAL / DOWN /
STRONG_DOWN / INSUFFICIENT_DATA for n<21). Body reports bars_used,
slope_window (20), obv_value, obv_slope, obv_change_pct, obv_min_20,
obv_max_20, last_close. Complements CMF (ADR-153): CMF is bounded
[−1, +1] and forgets old volume, OBV is unbounded and remembers all
history. Halted/zero-volume bars contribute zero (standard Granville),
so OBV can plateau during halts. Source: ADR-154 OBV window.

#### 2.215 Hutson Triple-EMA Rate-of-Change (TRIX — ADR-154)

Pulled from `research::get_trix`. Hutson (*Stocks & Commodities*, 1983)
triple-smoothed momentum oscillator at period=15 signal=9.
EMA3 = EMA(EMA(EMA(close, 15), 15), 15); TRIX = 100·(EMA3_t/EMA3_{t−1} − 1);
signal = EMA(TRIX, 9). Header gives **trix_label** (STRONG_BULL TRIX>0
&& TRIX>signal && |TRIX|>0.05 / BULL TRIX>0 / NEUTRAL / BEAR TRIX<0 /
STRONG_BEAR TRIX<0 && TRIX<signal && |TRIX|>0.05 / INSUFFICIENT_DATA
for n<55). Body reports bars_used, period (15), signal_period (9),
trix_value, signal_value, histogram, ema3_value, last_close.
Complements MACD (EMA-EMA spread) and TSI (double-smoothed): TRIX is the
highest-smoothing end of the momentum-oscillator spectrum — more noise
rejection, more lag. Source: ADR-154 TRIX window.

#### 2.216 Hull Moving Average (HMA — ADR-154)

Pulled from `research::get_hma`. Hull (2005) explicitly-least-lagged
weighted-MA construct at period=20. HMA = WMA(2·WMA(n/2) − WMA(n), √n).
Inner difference 2·WMA(10) − WMA(20) has near-zero lag; outer WMA(√20=4)
smooths the result. Header gives **hma_label** (STRONG_UP slope>2% /
UP slope>0.2% / NEUTRAL / DOWN slope<−0.2% / STRONG_DOWN slope<−2% /
INSUFFICIENT_DATA for n<21). Body reports bars_used, period (20),
half_period (10), sqrt_period (4), hma_value, hma_slope_pct,
hma_vs_close_pct, last_close. Complements SMA/EMA/KAMA (ADR-142): HMA
is the zero-lag-by-construction member of the MA family. We floor √n
(TradingView convention). Source: ADR-154 HMA window.

#### 2.217 Prior Ingested Web Research (INGESTED — ADR-130)

Pulled from `research::get_ingested_articles`. Emitted only when a
prior AI conversation has ingested web-search results for this
symbol — populated by sending articles from Claude / Gemini (CLI or
API) back into the terminal via the `INGEST_RESEARCH` command or
the `===TYPHOON_INGEST===` Return Path footer that the packet
builder now prints at the end of every research packet. Header
reports the number of articles in the bag; body emits the **top
15** articles (newest first) with title, source, published date,
`agent_used`, truncated summary (≤260 chars), and URL. The cache
holds up to 50 articles per symbol — FIFO with URL-based dedup,
timestamp-wins semantics — and LAN-syncs like every other research
table so a LAN client's ingestion populates the bag on all peers.
Source: ADR-130 INGEST_RESEARCH window + Return Path parser.

#### 2.218 Sector peer comparison

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

### 5. Return Path footer (ADR-130)

Every packet now closes with a **Return Path** instruction block that
asks the AI agent to echo any web-search articles it fetched back to
the terminal in a structured, parseable format. The terminal's
`INGEST_RESEARCH` command (and any future auto-ingest listener) scans
model replies for this block, parses the JSON, and appends the
articles to the per-symbol bag consumed by sub-block 2.217 above.

The footer is a fixed literal string — agents are told to emit:

```
===TYPHOON_INGEST===
[
  {
    "symbol": "AAPL",
    "title": "...",
    "url": "https://...",
    "source": "Reuters",
    "published_at": "2026-04-14",
    "summary": "...",
    "agent_used": "claude-opus-4-6"
  },
  ...
]
===END_INGEST===
```

The parser (`research::parse_ingest_block`) is intentionally lenient:
it accepts ` ```json ` fences around the array, accepts `published` /
`date` as aliases for `published_at`, accepts `agent` for
`agent_used`, skips entries missing `symbol` or `url`, and ignores
any text surrounding the sentinels. `agent_used` is overridden
post-parse with the ingest window's "agent tag" field when the user
wants to label a paste from a transcript.

The footer adds roughly **~600 bytes** per packet regardless of how
many symbols are in scope — it is emitted once after the closing
Question section, not per-symbol.

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
| Leverage ratios (ADR-117 LEV) | 6 rows | Debt/EBITDA, Net Debt/EBITDA, Debt/Equity, Interest Coverage, Current, Quick |
| Accruals periods (ADR-117 ACRL) | 8 quarters | 2-year cash-conversion trajectory |
| Realized vol windows (ADR-117 RVOL) | 4 rows | 20d / 60d / 120d / 252d cone |
| FCF yield periods (ADR-117 FCFY) | 6 rows | TTM + 5 annual rows for 5Y CAGR |
| Short interest headline fields (ADR-117 SHRT) | 7 k/v rows | Short%, DTC, float, shares out, ADV, short ratio, utilization |
| Altman Z components (ADR-118 ALTZ) | 5 rows | A/B/C/D/E with ratio + coefficient + contribution |
| Piotroski F-score checks (ADR-118 PTFS) | 9 rows | Fixed 9-point quality checklist |
| OHLC volatility estimators (ADR-118 VOLE) | 5 rows | CtC / Parkinson / GK / RS / YZ |
| EPS beat streak fields (ADR-118 EPSB) | 8 k/v rows | Counts, streaks, surprise avg/median/recent, latest |
| Price target dispersion fields (ADR-118 PTD) | 8 k/v rows | Targets, dispersion, spread, implied returns |
| Insider activity fields (ADR-119 MNGR) | 10 k/v rows | Counts, unique insiders, gross/net $, net shares, latest date |
| Dividend growth annual rows (ADR-119 DIVG) | ≤10 rows | Decade of calendar-year buckets |
| Earnings momentum quarters (ADR-119 EARM) | ≤8 rows | 8 quarters covers both recent and prior 4Q comparison windows |
| Sector rotation fields (ADR-119 SECTR) | 10 k/v rows | Sector rank, rel strength, breadth, strongest/weakest |
| Upgrade/downgrade momentum fields (ADR-119 UPDM) | 12 k/v rows | Net 30/90/180, counts per bucket, latest action |
| Momentum 12-1 fields (ADR-120 MOM) | 5 k/v rows | Regime, trend, composite, returns ladder, vol-adj score |
| Liquidity profile fields (ADR-120 LIQ) | 10 k/v rows | Tier, avg/median $, turnover, Amihud, ATR, spread proxy |
| Breakout proximity fields (ADR-120 BREAK) | 10 k/v rows | Label, setup, 20d/60d/52w ranges, position, consolidation |
| Cash conversion cycle periods (ADR-120 CCRL) | ≤8 rows | Latest period + up to 7 prior with DSO/DIO/DPO/CCC |
| Credit score components (ADR-120 CREDIT) | ≤6 rows | ALTZ / PTFS / LEV / ACRL each with value + score + weight |
| GARP composite fields (ADR-121 GROWM) | 5 k/v rows + ≤5 component rows | Label, score, inputs, MOM/EARM/DIVG sub-lines + component contributions |
| Smart-money flow fields (ADR-121 FLOW) | 10 k/v rows | Label, composite, insider+institutional sub-scores, buys/sells/net USD, trades+unique, buyers/sellers/holders, net ratio, share delta |
| Market regime fields (ADR-121 REGIME) | 8 k/v rows | Label, composite, realized vol + source, ADX + trend summary, 1Y return, Sharpe, 3 sub-scores, inputs |
| Relative volume fields (ADR-121 RELVOL) | 6 k/v rows | Activity, direction, current+avg, rel-vol ratios, 5d-vs-20d trend, 60d percentile, bars used |
| Margin trajectory rows (ADR-121 MARGINS) | 3×4 grid + ≤6 period rows | Gross/op/net margin (latest/prior/Δpp/trend) + avg row + per-period history |
| Value-factor composite fields (ADR-122 VAL) | 6 metric pairs + ≤6 component rows | Ratio vs sector median for P/E, Forward P/E, P/B, P/S, EV/EBITDA, FCF Yield |
| Quality-factor composite fields (ADR-122 QUAL) | 8 k/v rows + ≤4 component rows | Piotroski, op margin + trend, cash conversion + trend, leverage summary, debt/EBITDA |
| Risk-factor composite fields (ADR-122 RISK) | 7 k/v rows + ≤5 component rows | Realized vol, beta, liquidity tier, short%float + DTC, Altman Z + zone; higher = riskier |
| Insider streak rows (ADR-122 INSSTRK) | 8 k/v rows + ≤8 per-insider rows | Unique insiders, buy/sell streak counts, longest streaks, net $ totals |
| Coverage breadth fields (ADR-122 COVG) | 12 k/v rows | Num analysts, target mean/low/high, consensus 5-bucket, bull ratio, 90d churn, 3 sub-scores |
| Value rank fields (ADR-123 VRK) | 4 k/v rows | Subject composite + sector median/p25/p75 + percentile + rank position |
| Quality rank fields (ADR-123 QRK) | 4 k/v rows | Subject composite + sector median/p25/p75 + percentile + rank position |
| Risk rank fields (ADR-123 RRK) | 4 k/v rows | Subject composite (higher=riskier) + sector median/p25/p75 + SAFE percentile + rank position |
| Relative EPS growth fields (ADR-123 RELEPSGR) | 4 k/v rows | Latest/earliest EPS, sector median/p25/p75 CAGR, gap-to-median in pp |
| PEAD fields (ADR-123 PEAD) | 6 k/v rows + ≤8 event rows | Avg drift 1d/3d/5d/10d, BEAT/MISS breakouts, latest event, per-event detail table |
| Size factor fields (ADR-124 SIZEF) | 4 k/v rows | Market cap + log(cap) + sector median/p25/p75 caps + percentile + rank position |
| Momentum rank fields (ADR-124 MOMF) | 3 k/v rows | Subject momentum composite + sector median/p25/p75 + percentile + rank position |
| PEAD rank fields (ADR-124 PEADRANK) | 3 k/v rows | Subject avg 5d drift + sector median/p25/p75 + percentile + rank position |
| Fundamental quality meter fields (ADR-124 FQM) | 4 k/v rows | Piotroski, op margin + trend, cash conversion + trend, PTFS/MARGINS/ACRL components |
| Relative revenue growth fields (ADR-124 REVRANK) | 4 k/v rows | Latest/earliest revenue, sector median/p25/p75 CAGR, gap-to-median in pp |
| Leverage rank fields (ADR-125 LEVRANK) | 3 k/v rows | Subject D/E + sector median/p25/p75 D/E + SAFE percentile + rank position (NEGATIVE_EQUITY branch prints raw total_debt/total_equity) |
| Operating quality rank fields (ADR-125 OPERANK) | 3 k/v rows | Subject op margin % + trend + sector median/p25/p75 op margin + percentile + rank position |
| FQM rank fields (ADR-125 FQMRANK) | 3 k/v rows | Subject FQM composite + operator label + sector median/p25/p75 + percentile + rank position |
| Liquidity rank fields (ADR-125 LIQRANK) | 3 k/v rows | Subject ADV$ in $M + tier label + sector median/p25/p75 ADV$ + percentile + rank position |
| Earnings surprise streak fields (ADR-125 SURPSTK) | 4 k/v rows | Events breakdown + beat rate + current/longest streaks + avg surprise + latest event |
| Dividend growth rank fields (ADR-126 DVDRANK) | 4 k/v rows | Subject 3y CAGR + consecutive growth years + trend label + sector median/p25/p75 CAGR + percentile + rank position |
| Earnings momentum rank fields (ADR-126 EARMRANK) | 3 k/v rows | Subject composite score + momentum label + sector median/p25/p75 + percentile + rank position |
| Upgrade/downgrade rank fields (ADR-126 UPDGRANK) | 3 k/v rows | Subject net_90d + bias label + sector median/p25/p75 net + percentile + rank position |
| Gap yearly fields (ADR-126 GY) | 5 k/v rows | 253-bar gap census: 2/5/10% gap bins up+down + largest up/down with date + avg |gap| + label |
| Daily event streak fields (ADR-126 DES) | 4 k/v rows | 253-bar up/down/flat census + longest up/down streaks + current streak + up-day rate + avg up/down move |
| Dividend yield rank fields (ADR-127 DVDYIELDRANK) | 3 k/v rows | Subject yield + sector median/p25/p75 yield + percentile + rank position (non-payers filtered) |
| Short interest rank fields (ADR-127 SHRANK) | 3 k/v rows | Subject short % of float + sector median/p25/p75 short + risk-inverted percentile + rank position |
| Annualized ATR fields (ADR-127 ATRANN) | 4 k/v rows | Latest close + ATR14 price units + ATR14 % + annualized % (×√252) + volatility regime label |
| Drawdown history fields (ADR-127 DDHIST) | 5 k/v rows | Max drawdown % + peak/trough dates + longest drawdown days + 5%/10% correction counts + current drawdown |
| Price performance fields (ADR-127 PRICEPERF) | 4 k/v rows | 1M/3M/6M/YTD/1Y returns + latest close + bars used + trend label (1Y/3M blend) |
| Beta rank fields (ADR-128 BETARANK) | 3 k/v rows | Subject beta + sector median/p25/p75 + risk-inverted percentile + rank position (lower beta = safer) |
| PEG rank fields (ADR-128 PEGRANK) | 3 k/v rows | Subject PEG + sector median/p25/p75 + value-inverted percentile + rank position (lower PEG = better value) |
| 52-week high/low fields (ADR-128 FHIGHLOW) | 5 k/v rows | 52w high/low + dates + days since + pct-from-high/low + range position + proximity label |
| Realized vol cone fields (ADR-128 RVCONE) | 4 k/v rows | RV20/60/120/252 + RV20 rolling min/median/max + RV20 percentile + cone label |
| Calendar period breakdown fields (ADR-128 CALPB) | 5 k/v rows | MTD + QTD + YTD + prior quarter + prior year + current year/quarter + momentum label |
| Return skewness fields (ADR-129 RETSKEW) | 4 k/v rows | Bars used + mean/stdev log return + skewness + positive-day % + largest up/down + skew label |
| Return excess kurtosis fields (ADR-129 RETKURT) | 4 k/v rows | Bars used + stdev + excess kurtosis + 2σ/3σ outlier counts + 2σ outlier pct + kurt label |
| Tail ratio fields (ADR-129 TAILR) | 4 k/v rows | Bars used + P95/P05 + 95/5 tail ratio + P99/P01 + 99/1 tail ratio + bias label |
| Run length fields (ADR-129 RUNLEN) | 4 k/v rows | Bars used + avg up/down runs + run counts + longest up/down + signed current run + trend label |
| Daily range fields (ADR-129 DAYRANGE) | 4 k/v rows | Bars used + avg 60d/252d range % + latest range + compression ratio + widest/narrowest + range label |
| Return autocorrelation fields (ADR-131 AUTOCOR) | 3 k/v rows | Bars used + mean log return + ACF at lags 1/5/10/20 + regime label |
| Hurst exponent fields (ADR-131 HURST) | 2 k/v rows | Bars used + H + scales used + min/max scale + memory label |
| Hit rate fields (ADR-131 HITRATE) | 3 k/v rows | Bars used + up/down/flat days + hitrate 5d/20d/60d/252d + hit label |
| Gain/loss asymmetry fields (ADR-131 GLASYM) | 3 k/v rows | Bars used + up/down day counts + avg/median up & down pct + ratio + asymmetry label |
| Up/down volume ratio fields (ADR-131 VOLRATIO) | 4 k/v rows | Bars used + up/down day counts + avg/median up & down volume + ratio + max up/down volume + flow label |
| Rally history fields (ADR-132 DRAWUP) | 3 k/v rows | Bars used + max drawup % + trough/peak dates + longest rally days + rallies ≥5%/≥10% + current drawup + rally label |
| Overnight gap stat fields (ADR-132 GAPSTATS) | 3 k/v rows | Bars used + gap up/down counts + frequency + avg/up/down gap pct + largest up/down gap + bias label |
| Volatility clustering fields (ADR-132 VOLCLUSTER) | 2 k/v rows | Bars used + |r| ACF lag 1/5/20 + r² ACF lag 1/5/20 + cluster label |
| Close placement fields (ADR-132 CLOSEPLC) | 2 k/v rows | Bars used + avg/median/latest placement + near-high/near-low % + placement label |
| Mean-reversion half-life fields (ADR-132 MRHL) | 2 k/v rows | Bars used + AR(1) β/α + R² + half-life days + regime label |
| Downside deviation / Sortino fields (ADR-133 DOWNVOL) | 2 k/v rows | Bars used + mean log return + downside/upside dev + Sortino raw/ann + downside % of total var + sortino label |
| Sharpe ratio fields (ADR-133 SHARPR) | 2 k/v rows | Bars used + mean/stdev log return + Sharpe raw/ann + mean/stdev ann + sharpe label |
| Kaufman efficiency ratio fields (ADR-133 EFFRATIO) | 2 k/v rows | Bars used + start/end close + net change + Σ |Δclose| + ER + signed ER + efficiency label |
| Wick bias fields (ADR-133 WICKBIAS) | 2 k/v rows | Bars used + avg/median upper/lower wick + body share + bias score + bias label |
| Vol-of-vol fields (ADR-133 VOLOFVOL) | 2 k/v rows | RV points + mean/stdev/min/max/latest RV20 + CV + cv label |
| Calmar ratio fields (ADR-134 CALMAR) | 2 k/v rows | Bars used + total/annualized return + max drawdown + calmar ratio + calmar label |
| Ulcer index fields (ADR-134 ULCER) | 2 k/v rows | Bars used + ulcer index + mean/max drawdown + % in drawdown + ann return + Martin ratio + ulcer label |
| Variance ratio fields (ADR-134 VARRATIO) | 2 k/v rows | Bars used + VR(2/5/10/20) + z-stat(2/5) + rw label |
| Amihud illiquidity fields (ADR-134 AMIHUD) | 2 k/v rows | Bars used + mean/median/90th ILLIQ + avg $ volume + illiq label |
| Jarque-Bera normality fields (ADR-134 JBNORM) | 2 k/v rows | Bars used + skewness + excess kurtosis + JB statistic + p-value + normal label |
| Omega ratio fields (ADR-135 OMEGA) | 2 k/v rows | Bars used + gains/losses Σ + gain/loss days + win rate + omega ratio + omega label |
| Detrended fluctuation fields (ADR-135 DFA) | 2 k/v rows | Bars used + α exponent + num scales + log-log R² + dfa label |
| Burke ratio fields (ADR-135 BURKE) | 2 k/v rows | Bars used + annualized return + dd event count + Σdd² + worst event dd + burke ratio + burke label |
| Monthly seasonality fields (ADR-135 MONTHSEAS) | 3-4 k/v rows | Years covered + best/worst month + full 12-cell month grid (hit % + mean ret %) + season label |
| Roll implicit spread fields (ADR-135 ROLLSPRD) | 2 k/v rows | Bars used + first-lag cov + mean price + implicit spread + implicit spread (bps) + roll label |
| Parkinson vol fields (ADR-136 PARKINSON) | 2 k/v rows | Bars used + daily σ + annualized σ + mean ln(H/L) + vol label |
| Garman-Klass vol fields (ADR-136 GKVOL) | 2 k/v rows | Bars used + daily σ + annualized σ + range/C-O components + vol label |
| Rogers-Satchell vol fields (ADR-136 RSVOL) | 2 k/v rows | Bars used + daily σ + annualized σ (drift-independent) + vol label |
| CVaR / Expected Shortfall fields (ADR-136 CVAR) | 2 k/v rows | Bars used + VaR/ES(5%) + VaR/ES(1%) + tail day counts + cvar label |
| Day-of-week seasonality fields (ADR-136 DOWEFFECT) | 2-3 k/v rows | Bars used + weeks covered + best/worst weekday + full 5-cell weekday grid (hit % + mean ret %) + dow label |
| Sterling ratio fields (ADR-137 STERLING) | 2 k/v rows | Bars used + annualized return + worst_n + dd event count + mean worst-N dd + sterling ratio + sterling label |
| Kelly fraction fields (ADR-137 KELLYF) | 2 k/v rows | Bars used + win/loss rate + avg win/loss % + b ratio + f* + half_kelly + kelly label |
| Ljung-Box fields (ADR-137 LJUNGB) | 2 k/v rows | Bars used + lag h + Q-statistic + p-value + reject white noise + ljungb label |
| Runs test fields (ADR-137 RUNSTEST) | 2 k/v rows | Bars used + positive/negative days + runs observed/expected/std + z-stat + p-value + reject randomness + runs label |
| Zero-return fields (ADR-137 ZERORET) | 2 k/v rows | Bars used + zero day count + zero day % + longest zero streak + epsilon + zero label |
| Probabilistic Sharpe fields (ADR-138 PSR) | 2 k/v rows | Bars used + annualized Sharpe + skewness γ₃ + kurtosis γ₄ + SR benchmark + PSR + psr label |
| Dickey-Fuller fields (ADR-138 ADF) | 2 k/v rows | Bars used + β + SE(β) + t-statistic + crit 1/5/10% + reject unit root + adf label |
| Mann-Kendall fields (ADR-138 MNKENDALL) | 2 k/v rows | Bars used + S-statistic + variance + z-statistic + p-value + Kendall τ + reject no-trend + mk label |
| Bipower fields (ADR-138 BIPOWER) | 2 k/v rows | Bars used + realized variance + bipower variation + continuous/realized ann vol % + jump ratio + jump % + jump label |
| Drawdown duration fields (ADR-138 DDDUR) | 2 k/v rows | Bars used + event count + max/mean/median duration + total bars underwater + % underwater + currently underwater + current duration + dddur label |
| Hill tail-index fields (ADR-139 HILLTAIL) | 2 k/v rows | Returns used + k order stats + threshold |r|(k+1) + α on |r|/left/right + tail label |
| ARCH-LM fields (ADR-139 ARCHLM) | 2 k/v rows | Returns used + q lags + R² + LM=n·R² + p-value + crit χ²(5) 5%/1% + reject homoskedastic + arch label |
| Pain ratio fields (ADR-139 PAINRATIO) | 2 k/v rows | Bars used + pain index (mean\|dd\|) + annualized return + pain ratio + max dd + pain label |
| CUSUM break fields (ADR-139 CUSUM) | 2 k/v rows | Returns used + max\|S_t\| + D=max\|S_t\|/√n + bar at max + direction + crit 10/5/1% + reject stability + cusum label |
| Cornish-Fisher VaR fields (ADR-139 CFVAR) | 2 k/v rows | Returns used + mean/σ returns + skew γ₃ + excess kurt γ₄ + Gauss/CF VaR 5%+1% + adj 5% + skew/kurt term contributions + cfvar label |
| Shannon entropy fields (ADR-140 ENTROPY) | 2 k/v rows | Returns used + bins + H bits + H_max bits + normalised H/H_max + entropy label |
| Rachev ratio fields (ADR-140 RACHEV) | 2 k/v rows | Returns used + ES right/left 5% + Rachev 5% + ES right/left 1% + Rachev 1% + rachev label |
| Gain-to-Pain fields (ADR-140 GPR) | 2 k/v rows | Returns used + Σ all/gains/\|losses\| % + GPR + profit factor + wins/losses + gpr label |
| PACF fields (ADR-140 PACF) | 2 k/v rows | Returns used + PACF lags 1-5 + Bartlett 95% crit + sig lags + max \|PACF\| + max lag + pacf label |
| Approximate entropy fields (ADR-140 APEN) | 2 k/v rows | Returns used + m + r + Φ^m + Φ^(m+1) + ApEn + apen label |
| Upside potential ratio fields (ADR-141 UPR) | 2 k/v rows | Bars used + UPM₁ + LPM₂ + √LPM₂ + UPR + up/down days + upr label |
| Leverage effect fields (ADR-141 LEVEREFF) | 2 k/v rows | Bars used + corr(r,vol²) + down/up vol + asymmetry ratio + pairs used + levereff label |
| Drawdown-at-Risk fields (ADR-141 DRAWDAR) | 2 k/v rows | Bars used + max dd + DaR/CDaR 5% + DaR/CDaR 1% + dd events + drawdar label |
| Volatility half-life fields (ADR-141 VARHALF) | 2 k/v rows | RV points + AR(1) β/α + half-life days + mean/latest RV + varhalf label |
| Return Gini coefficient fields (ADR-141 GINI) | 2 k/v rows | Bars used + Gini + mean/max/min \|r\| + gini label |
| Sample entropy fields (ADR-142 SAMPEN) | 2 k/v rows | Returns used + m + r + A/B counts + SampEn + sampen label |
| Permutation entropy fields (ADR-142 PERMEN) | 2 k/v rows | Returns used + m + patterns obs/possible + H_raw + H_norm + permen label |
| Recovery factor fields (ADR-142 RECFACT) | 2 k/v rows | Bars used + cum return % + max dd % + recovery factor + recfact label |
| KPSS stationarity fields (ADR-142 KPSS) | 2 k/v rows | Returns used + η_μ + lag ℓ + crit 10/5/1% + reject_stationary + kpss label |
| Spectral entropy fields (ADR-142 SPECENT) | 2 k/v rows | Returns used + freq bins + H_raw + H_norm + peak idx + peak share + specent label |
| Robust volatility fields (ADR-143 ROBVOL) | 2 k/v rows | Returns used + classical/MAD/IQR σ (annual) + MAD ratio + IQR ratio + robvol label |
| Rényi entropy fields (ADR-143 RENYIENT) | 2 k/v rows | Returns used + bins + α + H₂ raw + H₂ normalised + collision_prob + renyient label |
| Return quantile profile fields (ADR-143 RETQUANT) | 2 k/v rows | Returns used + P1/P5/P10/P25/P50/P75/P90/P95/P99 + IQR + tail asymmetry + retquant label |
| Multiscale entropy fields (ADR-143 MSENT) | 2 k/v rows | Returns used + m + r + τ_max + SampEn τ=1..5 + complexity index + msent label |
| EWMA volatility fields (ADR-143 EWMAVOL) | 2 k/v rows | Returns used + λ + variance + σ daily/annual + classical σ annual + ewma/classical ratio + ewmavol label |
| KS normality test fields (ADR-144 KSNORM) | 2 k/v rows | Returns used + D statistic + 10%/5%/1% criticals + reject flags + μ + σ + ksnorm label |
| Anderson-Darling fields (ADR-144 ADTEST) | 2 k/v rows | Returns used + A² + A²_adj + p-value approx + 10%/5%/1% criticals + reject flags + adtest label |
| L-moments fields (ADR-144 LMOM) | 2 k/v rows | Returns used + L1/L2/L3/L4 + τ3 skew + τ4 kurt + lmom label |
| Kyle's λ fields (ADR-144 KYLELAM) | 2 k/v rows | Bars used + Kyle λ + mean \|Δp\| + mean V + correlation ρ + R² + kylelam label |
| Peaks-over-threshold fields (ADR-144 PEAKOVER) | 3 k/v rows | Returns used + P95/P99 thresholds + counts + mean/max excesses (both P95 and P99) + peakover label |
| Higuchi fractal dim fields (ADR-145 HIGUCHI) | 2 k/v rows | Returns used + k_max + fractal_dim + R² + log-k points + higuchi label |
| Pickands tail-index fields (ADR-145 PICKANDS) | 2 k/v rows | Returns used + k + γ̂ + tail α + x_k/x_2k/x_4k order-stats + pickands label |
| Kappa-3 ratio fields (ADR-145 KAPPA3) | 2 k/v rows | Returns used + MAR + excess μ + LPM3 + LPM3^(1/3) + κ3 + Sortino reference + kappa3 label |
| Lyapunov exponent fields (ADR-145 LYAPUNOV) | 2 k/v rows | Returns used + m + τ + λ_max + R² + steps used + lyapunov label |
| Spearman rank autocorrelation (ADR-145 RANKAC) | 2 k/v rows | Returns used + ρ(1) + ρ(5) + ρ(10) + mean\|ρ\| + max\|ρ\| + rankac label |
| BNS jump-test Z fields (ADR-146 BNSJUMP) | 2 k/v rows | Returns used + RV + BV + jump ratio + z-statistic + approx p-value + bnsjump label |
| Phillips-Perron fields (ADR-146 PPROOT) | 2 k/v rows | Bars used + ρ̂ + raw t + PP Z(ρ) + PP Z(t) + auto-picked lag truncation q + pproot label |
| Multifractal DFA fields (ADR-146 MFDFA) | 2 k/v rows | Returns used + h(−2) + h(0) + h(+2) + Δh + scales used + mfdfa label |
| Hill-tail KS fields (ADR-146 HILLKS) | 2 k/v rows | Returns used + k (tail size) + Hill α̂ + KS D statistic + KS 5% critical + hillks label |
| True Strength Index fields (ADR-146 TSI) | 2 k/v rows | Bars used + EMA long/short periods + TSI value + signal value + TSI−signal + tsi label |
| GARCH(1,1) fit fields (ADR-147 GARCH11) | 2 k/v rows | Returns used + ω + α + β + persistence + unconditional variance + half-life + log-likelihood + garch11 label |
| Sup-ADF bubble test fields (ADR-147 SADF) | 2 k/v rows | Bars used + min window r0 + full-ADF t + SADF stat + argmax end + critical 5% + reject null + sadf label |
| Correlation dimension fields (ADR-147 CORDIM) | 2 k/v rows | Returns used + embed_dim + radii fitted + D2 + fit R² + cordim label |
| Rolling skewness spectrum fields (ADR-147 SKSPEC) | 2 k/v rows | Returns used + window size + mean/std/min/max/range of rolling skew + skspec label |
| Auto mutual information fields (ADR-147 AUTOMI) | 2 k/v rows | Returns used + num_bins + MI(1/5/10) + H(X) + normalised MI(1)/H(X) + automi label |
| Ingested web articles (ADR-130 INGESTED) | 15 shown / 50 cached | Top 15 newest articles emitted per symbol; FIFO bag holds up to 50 with URL dedup + timestamp-wins replacement |
| Daily bars required for stats | ≥20 | Needed for 20d return and ATR warm-up |

There is no global packet size limit — total size scales with the number of
symbols. A single S&P 500 symbol now produces a packet around **~71-141 KB**
(up from 70-139 KB after ADR-153; ADR-154 adds five optional per-symbol
blocks — VORTEX / CHOP / OBV / TRIX / HMA — each measuring ~2 k/v rows
and adding ~220-260 bytes when populated, for a typical +1.2 KB per symbol;
all five reuse the existing `research_historical_price` HP cache and the
standard research-table LAN sync path with zero new API dependencies;
VORTEX computes the Botes & Siepman 2009 directional-movement alternative
to ADX at period=14 (VM+ = |H_t − L_{t−1}|, VM− = |L_t − H_{t−1}|,
VI± = ΣVM± / ΣTR) with BULL_CROSS (VI+>VI− with VI+>1) / BULL / NEUTRAL /
BEAR / BEAR_CROSS (VI−>VI+ with VI−>1) labels — the unsmoothed-and-
earlier direction-change complement to ADX's Wilder-smoothed trend-strength;
CHOP computes Dreiss 1980s bounded 0–100 trend-vs-range scalar at period=14
(100·log10(ΣTR / (maxH − minL)) / log10(N)) with CHOP (>61.8) / RANGING /
NEUTRAL / TRANSITIONAL / TRENDING (<38.2) labels using Fibonacci
complements — the *range-efficiency* bounded-by-construction complement
to ADX's unbounded trend-strength measurement; OBV computes Granville's
1963 cumulative sign(ΔClose)·Volume with a 20-bar linear-regression slope
normalised against OBV range to emit STRONG_UP / UP / NEUTRAL / DOWN /
STRONG_DOWN labels — the unbounded-all-history complement to CMF's
[−1, +1] bounded 20-bar-volume-forget window, closing the cumulative-vs-
period-based volume-indicator dimension (OBV 1963 predates CMF/MFI by
~20 years); TRIX computes Hutson's 1983 triple-smoothed momentum
oscillator at period=15 signal=9 (EMA3 = EMA(EMA(EMA(close, 15), 15), 15),
TRIX = 100·(EMA3_t/EMA3_{t−1} − 1), signal = EMA(TRIX, 9)) with
STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR labels — the
highest-smoothing end of the momentum-oscillator spectrum (MACD =
EMA-EMA, TSI = double-smoothed, TRIX = triple-smoothed), trading more
noise rejection for more lag; HMA computes Hull's 2005
explicitly-least-lagged MA construct at period=20 (HMA = WMA(2·WMA(n/2)
− WMA(n), √n)) with STRONG_UP (slope>2%) / UP / NEUTRAL / DOWN /
STRONG_DOWN (slope<−2%) labels — the zero-lag-by-construction member of
the MA family (SMA/EMA/KAMA), useful for fast turn-detection without
false-positive-ing on noise; prior five (ADR-153) remain unchanged;
ADR-153 adds five optional per-symbol
blocks — ADX / CCI / CMF / MFI / PSAR — each measuring ~2 k/v rows and
adding ~210-270 bytes when populated, for a typical +1.15 KB per symbol;
all five reuse the existing `research_historical_price` HP cache and the
standard research-table LAN sync path with zero new API dependencies;
ADX computes Wilder's 1978 directional-movement system at period=14
(+DM/−DM winner-takes-bar, Wilder-smoothed, normalised by ATR, ADX =
Wilder-smoothed DX) with STRONG_TREND (≥40) / TREND (≥25) / WEAK_TREND
(≥15) / NO_TREND labels — the trend-*strength* complement to AROON's
time-since-extreme measure; CCI computes Lambert's 1980 mean-deviation-
normalised oscillator (TP − SMA)/(0.015·MAD) at period=20 with
OVERBOUGHT (>100) / BULL / NEUTRAL / BEAR / OVERSOLD (<−100) labels —
mean-deviation normalisation gives different extremes than RSI's
gain/loss ratio on one-sided slow grinds; CMF computes Chaikin's
volume-weighted accumulation MFV = ((C−L) − (H−C))/(H−L)·volume summed
over 20 bars and normalised by volume sum to [−1, +1] with STRONG_ACCUM
(>0.25) / ACCUM / NEUTRAL / DIST / STRONG_DIST (<−0.25) labels — the
first volume-weighted accumulation-line surface, with H==L flat bars
epsilon-guarded to MFV=0; MFI computes Quong & Soudack's 1989
volume-weighted RSI at period=14 (raw money flow = TP × volume,
positive/negative by TP direction, ratio-based 100 − 100/(1+ratio))
with OVERBOUGHT (>80) / BULL / NEUTRAL / BEAR / OVERSOLD (<20) labels —
bars with heavy volume count more toward the oscillator than price-only
RSI; PSAR computes Wilder's accelerating trailing-stop (AF 0.02/0.02/0.20
with EP tracking and prior-two-bar flip clamp) with STRONG_UP / UP /
FLAT / DOWN / STRONG_DOWN labels — complements SUPERTREND by accelerating
in mature trends where SuperTrend's constant-multiplier ATR band does
not; prior five (ADR-152) remain unchanged; ADR-152 adds five optional
per-symbol blocks — ICHIMOKU / SUPERTREND / KELTNER / FISHER / AROON —
each measuring ~2 k/v rows and adding ~190-300 bytes when populated, for
a typical +1.2 KB per symbol; all five reuse the existing
`research_historical_price` HP cache and the standard research-table
LAN sync path with zero new API dependencies; ICHIMOKU computes the
canonical Japanese one-glance cloud (Tenkan-9, Kijun-26, Senkou A =
(T+K)/2, Senkou B-52, Chikou shifted back 26 bars) and labels by
close-vs-cloud position combined with Tenkan/Kijun cross direction
with STRONG_BULL / BULL / NEUTRAL / BEAR / STRONG_BEAR — the first
explicit Japanese-TA surface; SUPERTREND tracks a Wilder-ATR
trailing-stop band (period=10, multiplier=3) with strict flip
recursion comparing current close to the prior band value — the
regime-aware volatility-tracking complement to DONCHIAN's
event-based N-bar envelope breakout; KELTNER computes EMA-20 midline
± 2·ATR-10 channels with an inline Bollinger(20, 2σ) computed
internally so the surface can fire a **TTM-Squeeze** flag when BB is
fully inside KC (BB_upper ≤ KC_upper AND BB_lower ≥ KC_lower) —
John Carter's canonical volatility-compression / breakout-precursor
construct (*Mastering the Trade*, 2005), pairing KELTNER with
BBSQUEEZE (ADR-151) for the canonical TTM signal; FISHER applies
Ehlers' 2002 price-distribution transform 0.5·ln((1+x)/(1−x)) on hl2
midline rescaled over a 10-bar window to [−0.999, 0.999] with
0.66/0.67 smoothing + 0.5 prior feedback — PEAK_HIGH (>2) / BULL /
NEUTRAL / BEAR / PEAK_LOW (<−2) labels flag saturated regions about
to mean-revert; AROON computes Chande's 1995 time-since-extreme
oscillator at period=25 reporting Up/Down/Oscillator ∈ [−100, +100]
with STRONG_UP (osc>50) / WEAK_UP / CONSOLIDATION / WEAK_DOWN /
STRONG_DOWN labels — distinct from ADX/CHOP in measuring
*bars-since-new-extreme* rather than *trend-strength*, so fires the
moment a fresh 25-bar high/low prints; prior five (ADR-151) remain
unchanged; ADR-151 adds five optional per-symbol
blocks — SQUEEZE / SQUEEZERANK / BBSQUEEZE / DONCHIAN / KAMA — each
measuring ~2 k/v rows and adding ~180-320 bytes when populated, for
a typical +1.3 KB per symbol; four of the five reuse the existing
`research_historical_price` HP cache and SQUEEZE additionally reads
`research_short_interest` / `research_ivol` / `research_rvol` with the
standard research-table LAN sync path and zero new API dependencies;
SQUEEZE computes a composite 0..100 short-squeeze probability score over
five orthogonal axes (short % of float saturating at 40%, days-to-cover
saturating at 10 days, 20d price momentum saturating at 30%, relative
volume vs 20d average saturating at 3×, IV-rank 0..100) with 1.5× weight
on the mechanical axes (short-float + DTC) and 1.0× on the trigger axes
(momentum + relvol + IV-rank), re-normalised by active weight sum —
first multi-axis squeeze-screen surface with NO_SQUEEZE / WATCH /
ELEVATED / STRONG / EXTREME labels; SQUEEZERANK scans
`research_squeeze` across all symbols and emits TOP_1PCT / TOP_5PCT /
TOP_10PCT / ABOVE_MEDIAN / BELOW_MEDIAN labels driven by percentile —
the cross-symbol-rank complement to the single-symbol composite;
BBSQUEEZE computes 20-period Bollinger-band width (upper-lower)/mid and
ranks the current bar against its own trailing 120-bar history with
TIGHT_SQUEEZE (≤10th percentile) / MODERATE_SQUEEZE (≤25th) / NORMAL
/ EXPANSION (>75th) labels — the volatility-contraction complement to
the position-price SQUEEZE composite; DONCHIAN computes 20-bar
upper/lower channels using the *prior* window (excluding the current
bar to avoid self-reference) and flags BREAKOUT_UP / APPROACH_UP
(position ≥80) / NEUTRAL / APPROACH_DOWN (≤20) / BREAKOUT_DOWN — the
classical Turtle-Traders breakout surface; KAMA computes the Kaufman
Adaptive Moving Average with Efficiency Ratio = net move / path length
at n=10, fast=2, slow=30, reporting STRONG_TREND (ER>0.5) /
MODERATE_TREND (>0.3) / WEAK_TREND (>0.15) / CHOPPY labels — the
trend-quality complement to DONCHIAN's breakout detector (DONCHIAN
asks *did we break?*, KAMA asks *is the move clean enough to trade?*);
prior five (ADR-150) remain unchanged;
ADR-150 adds five optional per-symbol
blocks — MCLEODLI / OUFIT / GPH / BURGSPEC / KENDALLTAU — each
measuring ~2 k/v rows and adding ~200-280 bytes when populated, for
a typical +1.2 KB per symbol; all five reuse the existing
`research_historical_price` HP cache and the standard research-table
LAN sync path with zero new API dependencies; MCLEODLI runs the
McLeod-Li 1983 portmanteau Q = n(n+2) Σ ρ̂²(k)/(n-k) on squared returns
out to lag h = max(5, min(10, n/5)), compared against χ²(h) — a direct
ARCH-on-squared-returns diagnostic complementing ARCHLM and LJUNGB with
NO_ARCH / MILD_ARCH / STRONG_ARCH labels; OUFIT fits an OLS AR(1) on
log-prices and derives the continuous-time Ornstein-Uhlenbeck
parametrization θ = −ln(b), μ = a/(1−b), σ = residual sd, half-life =
ln(2)/θ — the first explicit SDE-parametrization surface, reporting
TRENDING / SLOW_REVERT / MODERATE_REVERT / FAST_REVERT labels driven
by the half-life-vs-window ratio; GPH computes the Geweke-Porter-Hudak
1983 semiparametric log-periodogram regression for the fractional
integration order d using m = floor(n^0.5) low frequencies, with
π²/(24m)-stderr t-test against H0: d=0 — the classical semiparametric
complement to HURST / DFA / HIGUCHI / MFDFA's fractal-dimension angles
with ANTIPERSISTENT / SHORT_MEMORY / LONG_MEMORY / NONSTATIONARY
labels; BURGSPEC fits an AR(p) with p = min(20, n/4) via the Burg
lattice recursion and evaluates the resulting spectral density on a
256-point grid — the parametric AR-spectrum complement to PERIODOGRAM's
non-parametric DFT with NO_AR_CYCLE / WEAK_AR_CYCLE / MODERATE_AR_CYCLE
/ STRONG_AR_CYCLE labels driven by peak-to-mean ratio; KENDALLTAU
computes the non-parametric Kendall tau lag-1 rank autocorrelation on
log-returns with asymptotic z-statistic — the rank-based complement to
DURBINWATSON's linear AR(1) and RANKAC's Spearman lag with STRONG_POS /
WEAK_POS / NO_RANK_AUTO / WEAK_NEG / STRONG_NEG labels; prior five
(ADR-149) remain unchanged;
up from 64-129 KB after ADR-146; ADR-147 adds five optional per-symbol
blocks — GARCH11 / SADF / CORDIM / SKSPEC / AUTOMI — each
measuring ~2 k/v rows and adding ~200-500 bytes when populated, for
a typical +1 KB per symbol and +2 KB worst case; all five reuse the
existing `research_historical_price` HP cache and the standard
research-table LAN sync path with zero new API dependencies;
GARCH11 fits Bollerslev 1986 σ²_t = ω + α·r²_{t-1} + β·σ²_{t-1}
via coordinate-descent grid MLE over (α, β) with ω implied by the
unconditional-variance constraint — first parametric volatility
persistence model in the terminal; persistence α+β and half-life
ln(0.5)/ln(α+β) are the key risk diagnostics, with
NEAR_INTEGRATED / HIGH / MODERATE / LOW_PERSISTENCE labels; SADF
computes the Phillips-Wu-Yu 2011 Sup-ADF statistic over an
expanding window from r0 = floor((0.01+1.8/√n)·n) forward,
comparing the sup-of-ADF-t against a tabulated 5% critical value
(interpolated in n) — first bubble / explosive-root detector,
complementing the three stationarity tests by asking the
asymmetric recent-tail question with EXPLOSIVE_CONFIRMED /
EXPLOSIVE_LIKELY / BORDERLINE / STABLE labels; CORDIM computes the
Grassberger-Procaccia 1983 correlation dimension D2 via
m=3 embedding and 10 log-spaced radii — first nonlinear-dynamics
dimension surface distinct from the monofractal scaling exponents
(Hurst/DFA/Higuchi); LOW_DIM (<1.5) suggests proximity to a
low-dimensional attractor, STOCHASTIC (≥3.5) suggests near-random
behaviour; SKSPEC rolls a 30-bar window over returns and reports
mean/std/min/max/range of the rolling skew — first
skewness-stability diagnostic, complementing RETQUANT's
full-window skew with STABLE_POSITIVE / STABLE_NEGATIVE /
DRIFTING / UNSTABLE labels driven by |mean|/std ratio; AUTOMI
computes auto-mutual-information MI(k) at lags 1/5/10 via k=8
equiprobable histogram bins plus H(X) and MI(1)/H(X) — first
information-theoretic ACF, catching nonlinear dependence invisible
to classical ACF with STRONG / MODERATE / WEAK / INDEPENDENT
labels driven by the MI(1)/H(X) ratio; prior five (ADR-146) remain
unchanged;
BNSJUMP computes the Barndorff-Nielsen-Shephard 2006 jump-test
Z-statistic z = (RV − BV) / sqrt(θ · Σr⁴) with an approximate p-value,
the first formal jump-detection hypothesis test (complementing Round
30's raw BIPOWER two-statistic comparison) — STRONG_JUMP / MODERATE /
WEAK / NO_JUMP labels driven by conventional normal critical values;
PPROOT computes the Phillips-Perron 1988 nonparametric unit-root
test via Newey-West corrections to OLS with Schwert-rule auto lag
q = floor(4·(n/100)^0.25), adding a third stationarity axis alongside
ADF and KPSS that is robust to conditional heteroscedasticity —
three-way ADF/KPSS/PP agreement is a much stronger stationarity
signal than any single test; MFDFA computes the Kantelhardt 2002
multifractal DFA spectrum h(q) at q ∈ {−2, 0, +2} over 7 scales
{8,12,16,24,32,48,64}, giving Δh = h(−2) − h(+2) as a width
diagnostic — the first multifractal-spectrum surface, complementing
monofractal HURST / DFA / HIGUCHI; HILLKS computes the KS
goodness-of-fit between the empirical tail distribution and the
Pareto model fitted by the Hill estimator at k = floor(n·0.10),
catching cases where HILLTAIL's α̂ is quantitative nonsense because
the tail shape doesn't actually fit a Pareto — GOOD_FIT /
ACCEPTABLE_FIT / POOR_FIT / REJECT labels driven by the 1.36/√k
critical value at 5%; TSI computes the Blau 1991 True Strength Index
= 100 · EMA₁₃(EMA₂₅(ΔP)) / EMA₁₃(EMA₂₅(|ΔP|)) with a short-EMA signal
line, providing a double-smoothed momentum oscillator with cleaner
zero-line behaviour than RSI and less lag than MACD); a 10-symbol
basket now lands near **710-1410 KB**
when every symbol has a fully populated ingest bag (the global
context and the Return Path footer are each emitted exactly once,
so multi-symbol overhead is still bounded by the per-symbol blocks).

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
| `research::get_leverage` | SQLite `research_leverage` | ADR-117 LEV window |
| `research::get_accruals` | SQLite `research_accruals` | ADR-117 ACRL window |
| `research::get_realized_vol` | SQLite `research_realized_vol` | ADR-117 RVOL window |
| `research::get_fcf_yield` | SQLite `research_fcf_yield` | ADR-117 FCFY window |
| `research::get_short_interest` | SQLite `research_short_interest` | ADR-117 SHRT window |
| `research::get_altman_z` | SQLite `research_altman_z` | ADR-118 ALTZ window |
| `research::get_piotroski` | SQLite `research_piotroski` | ADR-118 PTFS window |
| `research::get_ohlc_vol` | SQLite `research_ohlc_vol` | ADR-118 VOLE window |
| `research::get_eps_beat` | SQLite `research_eps_beat` | ADR-118 EPSB window |
| `research::get_price_target_dispersion` | SQLite `research_price_target_dispersion` | ADR-118 PTD window |
| `research::get_insider_activity` | SQLite `research_insider_activity` | ADR-119 MNGR window |
| `research::get_divg` | SQLite `research_divg` | ADR-119 DIVG window |
| `research::get_earm` | SQLite `research_earm` | ADR-119 EARM window |
| `research::get_sector_rotation` | SQLite `research_sector_rotation` | ADR-119 SECTR window |
| `research::get_updm` | SQLite `research_updm` | ADR-119 UPDM window |
| `research::get_momentum` | SQLite `research_momentum` | ADR-120 MOM window |
| `research::get_liquidity` | SQLite `research_liquidity` | ADR-120 LIQ window |
| `research::get_breakout` | SQLite `research_breakout` | ADR-120 BREAK window |
| `research::get_cash_cycle` | SQLite `research_cash_cycle` | ADR-120 CCRL window |
| `research::get_credit` | SQLite `research_credit` | ADR-120 CREDIT window |
| `research::get_growm` | SQLite `research_growm` | ADR-121 GROWM window (fuses MOM+EARM+DIVG) |
| `research::get_flow` | SQLite `research_flow` | ADR-121 FLOW window (composes cached INS+HDS) |
| `research::get_regime` | SQLite `research_regime` | ADR-121 REGIME window (fuses VOLE+TECH+HRA) |
| `research::get_relvol` | SQLite `research_relvol` | ADR-121 RELVOL window (over cached HP bars) |
| `research::get_margins` | SQLite `research_margins` | ADR-121 MARGINS window (over cached FA statements) |
| `research::get_val` | SQLite `research_val` | ADR-122 VAL window (fuses 6 valuation ratios vs sector-peer medians) |
| `research::get_qual` | SQLite `research_qual` | ADR-122 QUAL window (fuses PTFS+MARGINS+ACRL+LEV) |
| `research::get_risk` | SQLite `research_risk` | ADR-122 RISK window (fuses VOLE+BETA+LIQ+SHRT+ALTZ, higher = riskier) |
| `research::get_insstrk` | SQLite `research_insstrk` | ADR-122 INSSTRK window (groups cached InsiderTrade rows by insider) |
| `research::get_covg` | SQLite `research_covg` | ADR-122 COVG window (fuses PriceTarget+AnalystRecs+UPDM) |
| `research::get_vrk` | SQLite `research_vrk` | ADR-123 VRK window (sector percentile rank of VAL composite) |
| `research::get_qrk` | SQLite `research_qrk` | ADR-123 QRK window (sector percentile rank of QUAL composite) |
| `research::get_rrk` | SQLite `research_rrk` | ADR-123 RRK window (sector percentile rank of RISK composite, inverted) |
| `research::get_relepsgr` | SQLite `research_relepsgr` | ADR-123 RELEPSGR window (3y EPS CAGR vs sector median CAGR) |
| `research::get_pead` | SQLite `research_pead` | ADR-123 PEAD window (joins EarningsSurprise cache with HP bars) |
| `research::get_sizef` | SQLite `research_sizef` | ADR-124 SIZEF window (sector percentile rank of log market cap + absolute tier) |
| `research::get_momf` | SQLite `research_momf` | ADR-124 MOMF window (sector percentile rank of MOMENTUM composite) |
| `research::get_peadrank` | SQLite `research_peadrank` | ADR-124 PEADRANK window (sector percentile rank of PEAD avg 5d drift) |
| `research::get_fqm` | SQLite `research_fqm` | ADR-124 FQM window (fuses PTFS+MARGINS+ACRL, excludes leverage) |
| `research::get_revrank` | SQLite `research_revrank` | ADR-124 REVRANK window (3y revenue CAGR vs sector median CAGR) |
| `research::get_levrank` | SQLite `research_levrank` | ADR-125 LEVRANK window (sector percentile rank of D/E, risk-inverted, NEGATIVE_EQUITY short-circuit) |
| `research::get_operank` | SQLite `research_operank` | ADR-125 OPERANK window (sector percentile rank of operating margin) |
| `research::get_fqmrank` | SQLite `research_fqmrank` | ADR-125 FQMRANK window (sector percentile rank of FQM composite) |
| `research::get_liqrank` | SQLite `research_liqrank` | ADR-125 LIQRANK window (sector percentile rank of ADV$) |
| `research::get_surpstk` | SQLite `research_surpstk` | ADR-125 SURPSTK window (earnings surprise streak stat from cached EarningsSurprise rows) |
| `research::get_dvdrank` | SQLite `research_dvdrank` | ADR-126 DVDRANK window (sector percentile rank of 3y dividend CAGR) |
| `research::get_earmrank` | SQLite `research_earmrank` | ADR-126 EARMRANK window (sector percentile rank of EARM composite score) |
| `research::get_updgrank` | SQLite `research_updgrank` | ADR-126 UPDGRANK window (sector percentile rank of UPDM net_90d) |
| `research::get_gy` | SQLite `research_gy` | ADR-126 GY window (253-bar gap yearly stat from cached HP bars) |
| `research::get_des` | SQLite `research_des` | ADR-126 DES window (253-bar daily event streak stat from cached HP bars) |
| `research::get_dvdyieldrank` | SQLite `research_dvdyieldrank` | ADR-127 DVDYIELDRANK window (sector percentile rank of dividend yield, non-payers filtered) |
| `research::get_shrank` | SQLite `research_shrank` | ADR-127 SHRANK window (risk-inverted sector percentile rank of short_percent_of_float) |
| `research::get_atrann` | SQLite `research_atrann` | ADR-127 ATRANN window (Wilder 14-period ATR annualized via √252 with volatility regime label) |
| `research::get_ddhist` | SQLite `research_ddhist` | ADR-127 DDHIST window (253-bar drawdown history with max dd + longest dd + 5%/10% correction counts) |
| `research::get_priceperf` | SQLite `research_priceperf` | ADR-127 PRICEPERF window (multi-horizon total returns: 1M/3M/6M/YTD/1Y with trend label) |
| `research::get_betarank` | SQLite `research_betarank` | ADR-128 BETARANK window (risk-inverted sector percentile rank of Fundamentals.beta) |
| `research::get_pegrank` | SQLite `research_pegrank` | ADR-128 PEGRANK window (value-inverted sector percentile rank of Fundamentals.peg_ratio) |
| `research::get_fhighlow` | SQLite `research_fhighlow` | ADR-128 FHIGHLOW window (52-week high/low distance + proximity band) |
| `research::get_rvcone` | SQLite `research_rvcone` | ADR-128 RVCONE window (multi-horizon realized volatility cone 20d/60d/120d/252d) |
| `research::get_calpb` | SQLite `research_calpb` | ADR-128 CALPB window (calendar period breakdowns MTD/QTD/YTD + prior quarter/year) |
| `research::get_retskew` | SQLite `research_retskew` | ADR-129 RETSKEW window (Fisher-Pearson skewness of log returns + positive-day share) |
| `research::get_retkurt` | SQLite `research_retkurt` | ADR-129 RETKURT window (excess kurtosis + 2σ/3σ outlier counts) |
| `research::get_tailr` | SQLite `research_tailr` | ADR-129 TAILR window (95/5 and 99/1 quantile tail ratios) |
| `research::get_runlen` | SQLite `research_runlen` | ADR-129 RUNLEN window (up/down run length stats + signed current run) |
| `research::get_dayrange` | SQLite `research_dayrange` | ADR-129 DAYRANGE window (60d vs 252d daily range compression ratio) |
| `research::get_autocor` | SQLite `research_autocor` | ADR-131 AUTOCOR window (lag 1/5/10/20 return autocorrelation + momentum/mean-revert regime label) |
| `research::get_hurst` | SQLite `research_hurst` | ADR-131 HURST window (Hurst exponent via R/S analysis with 5-way persistence label) |
| `research::get_hitrate` | SQLite `research_hitrate` | ADR-131 HITRATE window (multi-horizon hit rate 5d/20d/60d/252d + up/down/flat counts + bias label) |
| `research::get_glasym` | SQLite `research_glasym` | ADR-131 GLASYM window (gain/loss magnitude asymmetry ratio + up/down day medians) |
| `research::get_volratio` | SQLite `research_volratio` | ADR-131 VOLRATIO window (up-day vs down-day volume ratio with accumulation/distribution label) |
| `research::get_drawup` | SQLite `research_drawup` | ADR-132 DRAWUP window (rally history mirror of DDHIST — deepest advance, longest rally, 5%/10% rally counts, current drawup with 5-way rally label) |
| `research::get_gapstats` | SQLite `research_gapstats` | ADR-132 GAPSTATS window (overnight gap frequency + magnitude with up/down bias label; first HP surface to read `bar.open`) |
| `research::get_volcluster` | SQLite `research_volcluster` | ADR-132 VOLCLUSTER window (ACF of r² and |r| at lags 1/5/20 — canonical GARCH-effect volatility clustering test with 5-way label) |
| `research::get_closeplc` | SQLite `research_closeplc` | ADR-132 CLOSEPLC window (average `(close-low)/(high-low)` bar-anatomy stat + near-high/near-low shares with bull/bear label) |
| `research::get_mrhl` | SQLite `research_mrhl` | ADR-132 MRHL window (AR(1) mean-reversion half-life via OLS fit on log returns with fast-revert/persistent label) |
| `research::get_downvol` | SQLite `research_downvol` | ADR-133 DOWNVOL window (downside deviation + Sortino ratio with 5-way risk-quality label) |
| `research::get_sharpr` | SQLite `research_sharpr` | ADR-133 SHARPR window (Sharpe ratio rf=0 raw + annualized with 5-way performance label) |
| `research::get_effratio` | SQLite `research_effratio` | ADR-133 EFFRATIO window (Kaufman efficiency ratio — net/gross price travel with trending/chopping label) |
| `research::get_wickbias` | SQLite `research_wickbias` | ADR-133 WICKBIAS window (upper vs lower wick share asymmetry with buyer/seller rejection label) |
| `research::get_volofvol` | SQLite `research_volofvol` | ADR-133 VOLOFVOL window (CV of rolling 20d realized vol — stable/chaotic vol-regime label) |
| `research::get_calmar` | SQLite `research_calmar` | ADR-134 CALMAR window (Calmar ratio — annualized return / max drawdown with 5-way label) |
| `research::get_ulcer` | SQLite `research_ulcer` | ADR-134 ULCER window (Ulcer index + Martin ratio — continuous drawdown-weighted risk with pain-level label) |
| `research::get_varratio` | SQLite `research_varratio` | ADR-134 VARRATIO window (Lo-MacKinlay variance ratio at horizons 2/5/10/20 — formal random-walk hypothesis test) |
| `research::get_amihud` | SQLite `research_amihud` | ADR-134 AMIHUD window (Amihud illiquidity ratio — |r|/dollar_volume microstructure liquidity scalar) |
| `research::get_jbnorm` | SQLite `research_jbnorm` | ADR-134 JBNORM window (Jarque-Bera normality test — combined skewness+kurtosis χ²(2) test with exact p-value) |
| `research::get_omega` | SQLite `research_omega` | ADR-135 OMEGA window (Omega ratio at threshold 0 — distribution-free gains/losses partition with 5-way label) |
| `research::get_dfa` | SQLite `research_dfa` | ADR-135 DFA window (Detrended Fluctuation Analysis α — Hurst alternative robust to non-stationarity) |
| `research::get_burke` | SQLite `research_burke` | ADR-135 BURKE window (Burke ratio — event-weighted drawdown-adjusted annualized return with 5-way label) |
| `research::get_monthseas` | SQLite `research_monthseas` | ADR-135 MONTHSEAS window (monthly seasonality — 12-month hit rate + mean return grid; full-HP-cache scan, not 253-window) |
| `research::get_rollsprd` | SQLite `research_rollsprd` | ADR-135 ROLLSPRD window (Roll's 1984 implicit bid-ask spread in bps — microstructure companion to AMIHUD) |
| `research::get_parkinson` | SQLite `research_parkinson` | ADR-136 PARKINSON window (Parkinson 1980 H-L range-based vol — 5.2× more efficient than close-to-close; first entry of the OHLC-vol family) |
| `research::get_gkvol` | SQLite `research_gkvol` | ADR-136 GKVOL window (Garman-Klass 1980 OHLC vol — H-L range + C-O drift, 7.4× efficiency; the textbook industrial range-vol estimator) |
| `research::get_rsvol` | SQLite `research_rsvol` | ADR-136 RSVOL window (Rogers-Satchell 1991 drift-independent OHLC vol — unbiased under non-zero drift; completes the OHLC-vol family) |
| `research::get_cvar` | SQLite `research_cvar` | ADR-136 CVAR window (Conditional VaR / Expected Shortfall at 5% and 1% — coherent Basel III downside-risk measure distinct from TAILR shape + DOWNVOL scale) |
| `research::get_doweffect` | SQLite `research_doweffect` | ADR-136 DOWEFFECT window (day-of-week intraday O→C seasonality — weekday calendar companion to MONTHSEAS; full-HP-cache scan) |
| `research::get_sterling` | SQLite `research_sterling` | ADR-137 STERLING window (Sterling ratio — annualized return over mean of N worst distinct drawdown events; middle ground between CALMAR single-worst and BURKE sum-of-squares) |
| `research::get_kellyf` | SQLite `research_kellyf` | ADR-137 KELLYF window (Kelly fraction `(b·p − q)/b` — forward-looking optimal-leverage scalar; first non-realized packet surface) |
| `research::get_ljungb` | SQLite `research_ljungb` | ADR-137 LJUNGB window (Ljung-Box Q at lag 10 — joint autocorrelation / white-noise test with Wilson-Hilferty χ²(10) p-value) |
| `research::get_runstest` | SQLite `research_runstest` | ADR-137 RUNSTEST window (Wald-Wolfowitz runs test — formal inferential randomness test on sign sequence with z-stat + two-sided p-value via A&S 7.1.26 normal CDF) |
| `research::get_zeroret` | SQLite `research_zeroret` | ADR-137 ZERORET window (Lesmond-Ogden-Trzcinka zero-return-day fraction — microstructure liquidity proxy distinct from AMIHUD impact + ROLLSPRD spread) |
| `research::get_psr` | SQLite `research_psr` | ADR-138 PSR window (Probabilistic Sharpe Ratio — Lopez de Prado 2012; corrects SHARPR for skewness/kurtosis and reports a probability the true SR exceeds a benchmark) |
| `research::get_adf` | SQLite `research_adf` | ADR-138 ADF window (Dickey-Fuller unit-root test on log-price with hardcoded MacKinnon 1996 critical values; rejection ⇒ stationary / mean-reverting) |
| `research::get_mnkendall` | SQLite `research_mnkendall` | ADR-138 MNKENDALL window (Mann-Kendall nonparametric trend test — distribution-free z-statistic over all i<j sign comparisons; complements ADF with trend-presence instead of stationarity) |
| `research::get_bipower` | SQLite `research_bipower` | ADR-138 BIPOWER window (Barndorff-Nielsen & Shephard 2004 bipower variation; jump_ratio = 1 − BPV/RV decomposes realized variance into continuous + jump components) |
| `research::get_dddur` | SQLite `research_dddur` | ADR-138 DDDUR window (drawdown duration statistics: event count + max/mean/median bar-duration + % of time underwater — duration-axis companion to CALMAR/BURKE/STERLING magnitude) |
| `research::get_hilltail` | SQLite `research_hilltail` | ADR-139 HILLTAIL window (Hill 1975 tail-index estimator on \|r\| plus separate left/right-tail α — nonparametric power-law exponent well-defined under infinite-variance heavy tails where KURT/JBNORM fail) |
| `research::get_archlm` | SQLite `research_archlm` | ADR-139 ARCHLM window (Engle 1982 ARCH Lagrange-multiplier test at q=5 lags — formal conditional-heteroskedasticity test with χ²(5) critical values; first packet surface on second-moment memory) |
| `research::get_painratio` | SQLite `research_painratio` | ADR-139 PAINRATIO window (Zephyr/FIBA Pain Index = mean\|dd\|, Pain Ratio = ann_return/pain_index — L¹ drawdown norm completing the magnitude-norm sextet with CALMAR/BURKE/STERLING/ULCER/DDDUR) |
| `research::get_cusum` | SQLite `research_cusum` | ADR-139 CUSUM window (Brown-Durbin-Evans 1975 OLS CUSUM structural-break test — first formal mean-stability test in the packet with KS critical values {10%=1.22, 5%=1.36, 1%=1.63}) |
| `research::get_cfvar` | SQLite `research_cfvar` | ADR-139 CFVAR window (Cornish-Fisher 1938 modified VaR — Gauss quantile adjusted for skew γ₃ and excess kurt γ₄ with skew-term vs kurt-term attribution) |
| `research::get_entropy` | SQLite `research_entropy` | ADR-140 ENTROPY window (Shannon entropy H = −Σ pᵢ log₂(pᵢ) over return histogram — first information-theoretic distributional measure with normalised H/H_max ∈ [0,1]) |
| `research::get_rachev` | SQLite `research_rachev` | ADR-140 RACHEV window (Rachev ratio = ES_α(+R)/ES_α(−R) at 5% and 1% — first asymmetric tail comparison ratio; Rachev > 1 ⇒ upside tail outweighs downside) |
| `research::get_gpr` | SQLite `research_gpr` | ADR-140 GPR window (Gain-to-Pain Ratio = Σ rₜ / Σ \|min(rₜ,0)\| + Profit Factor — Schwager's return-per-realized-loss metric, distinct from drawdown-based Pain Ratio) |
| `research::get_pacf` | SQLite `research_pacf` | ADR-140 PACF window (partial autocorrelation at lags 1–5 via Durbin-Levinson with Bartlett 95% band — first lag-specific dependence decomposition of LJUNGB joint autocorrelation) |
| `research::get_apen` | SQLite `research_apen` | ADR-140 APEN window (Pincus 1991 approximate entropy, m=2, r=0.2·σ — first nonlinear predictability measure capturing short-range pattern regularity) |
| `research::get_upr` | SQLite `research_upr` | ADR-141 UPR window (Sortino & van der Meer 1991 Upside Potential Ratio = UPM₁/√LPM₂ — first asymmetric capture-vs-risk ratio) |
| `research::get_levereff` | SQLite `research_levereff` | ADR-141 LEVEREFF window (Black 1976 leverage effect: corr(rₜ, rₜ₊₁²) + asymmetric vol ratio — first return→vol feedback measure) |
| `research::get_drawdar` | SQLite `research_drawdar` | ADR-141 DRAWDAR window (Chekhlov et al. 2005 Drawdown-at-Risk + CDaR at 5%/1% — first quantile-based drawdown risk measure) |
| `research::get_varhalf` | SQLite `research_varhalf` | ADR-141 VARHALF window (AR(1) on rolling 20d RV → half-life = −ln(2)/ln(β) — first vol-regime persistence measure) |
| `research::get_gini` | SQLite `research_gini` | ADR-141 GINI window (Gini coefficient on |log returns| — first return-concentration measure orthogonal to KURT/VOLCLUSTER/BIPOWER) |
| `research::get_sampen` | SQLite `research_sampen` | ADR-142 SAMPEN window (Richman & Moorman 2000 Sample Entropy, m=2, r=0.2·σ, self-match-excluded — modern standard complement to APEN) |
| `research::get_permen` | SQLite `research_permen` | ADR-142 PERMEN window (Bandt & Pompe 2002 Permutation Entropy, m=3 ordinal patterns — temporal ordering structure invisible to ENTROPY/APEN/SAMPEN) |
| `research::get_recfact` | SQLite `research_recfact` | ADR-142 RECFACT window (Recovery Factor = cum return / \|max dd\| — first raw-cumulative recovery metric distinct from annualized ratios) |
| `research::get_kpss` | SQLite `research_kpss` | ADR-142 KPSS window (Kwiatkowski-Phillips-Schmidt-Shin 1992 stationarity test — formal complement to ADF unit-root test) |
| `research::get_specent` | SQLite `research_specent` | ADR-142 SPECENT window (Spectral Entropy via DFT — Shannon entropy of normalised PSD, first frequency-domain periodicity measure) |
| `research::get_robvol` | SQLite `research_robvol` | ADR-143 ROBVOL window (MAD/0.6745 + IQR/1.349 robust σ + classical σ — first outlier-resistant vol surface, exposes classical-σ inflation by extreme days) |
| `research::get_renyient` | SQLite `research_renyient` | ADR-143 RENYIENT window (Rényi entropy at α=2, collision entropy, Σ pᵢ² concentration — first quadratic-order entropy, quadratic probability weighting vs Shannon's log) |
| `research::get_retquant` | SQLite `research_retquant` | ADR-143 RETQUANT window (9-point return quantile profile P1..P99 + IQR + tail asymmetry — first dense non-parametric distribution snapshot) |
| `research::get_msent` | SQLite `research_msent` | ADR-143 MSENT window (Costa-Goldberger-Peng 2005 Multiscale SampEn at τ=1..5 with fixed tolerance — first scale-dependent complexity measure) |
| `research::get_ewmavol` | SQLite `research_ewmavol` | ADR-143 EWMAVOL window (RiskMetrics EWMA variance with λ=0.94 — first adaptive-weighted vol surface with ewma/classical ratio as regime flag) |
| `research::get_ksnorm` | SQLite `research_ksnorm` | ADR-144 KSNORM window (Kolmogorov-Smirnov one-sample normality test against N(μ̂,σ̂²) with three-way 10%/5%/1% rejection flags — first omnibus goodness-of-fit surface) |
| `research::get_adtest` | SQLite `research_adtest` | ADR-144 ADTEST window (Anderson-Darling tail-weighted normality test with Stephens 1986 small-sample-corrected A² and p-value approximation — first tail-focused normality surface) |
| `research::get_lmom` | SQLite `research_lmom` | ADR-144 LMOM window (Hosking 1990 L-moments L1..L4 + L-ratios τ3 τ4 via unbiased probability-weighted moments — first robust-moment surface with bounded L-skew/L-kurt) |
| `research::get_kylelam` | SQLite `research_kylelam` | ADR-144 KYLELAM window (Kyle 1985 price-impact λ = cov(\|Δp\|,V)/var(V) regression slope + correlation + R² — first linear-regression liquidity surface, distinct from AMIHUD ratio) |
| `research::get_peakover` | SQLite `research_peakover` | ADR-144 PEAKOVER window (Peaks-Over-Threshold exceedance counts + mean/max excesses at P95 and P99 thresholds — first EVT/GPD-foundation surface) |
| `research::get_higuchi` | SQLite `research_higuchi` | ADR-145 HIGUCHI window (Higuchi 1988 fractal dimension of cumulative log-return walk, k_max=10, with SMOOTH/PERSISTENT/RANDOM/ROUGH regime labels — first direct geometric FD surface) |
| `research::get_pickands` | SQLite `research_pickands` | ADR-145 PICKANDS window (Pickands 1975 extreme-value γ̂ valid across Fréchet/Gumbel/Weibull domains — first EV-domain-agnostic tail estimator, cross-checks HILLTAIL Hill α) |
| `research::get_kappa3` | SQLite `research_kappa3` | ADR-145 KAPPA3 window (Kaplan-Knowles 2004 κ3 = (μ−MAR)/LPM3^(1/3) annualised with Sortino reference — first third-moment downside ratio) |
| `research::get_lyapunov` | SQLite `research_lyapunov` | ADR-145 LYAPUNOV window (Rosenstein 1993 largest Lyapunov exponent λ₁ with m=3 embedding, Theiler=10 — first chaos/nonlinear-dynamics surface) |
| `research::get_rankac` | SQLite `research_rankac` | ADR-145 RANKAC window (Spearman rank autocorrelation at lags 1/5/10 via average-rank tie handling — robust nonparametric counterpart to PACF) |
| `research::get_bnsjump` | SQLite `research_bnsjump` | ADR-146 BNSJUMP window (Barndorff-Nielsen-Shephard 2006 jump-test Z-statistic with approximate p-value — formal hypothesis-test version of Round 30's raw BIPOWER surface) |
| `research::get_pproot` | SQLite `research_pproot` | ADR-146 PPROOT window (Phillips-Perron 1988 nonparametric unit-root test with Schwert-rule auto lag truncation — third stationarity axis alongside ADF and KPSS, robust to conditional heteroscedasticity) |
| `research::get_mfdfa` | SQLite `research_mfdfa` | ADR-146 MFDFA window (Kantelhardt 2002 multifractal DFA at q ∈ {−2, 0, +2} over 7 scales with Δh spectrum width — first multifractal-spectrum surface) |
| `research::get_hillks` | SQLite `research_hillks` | ADR-146 HILLKS window (KS goodness-of-fit for Hill-tail Pareto model with k=floor(n·0.10) tail size — catches misspecified tail assumptions that HILLTAIL alone cannot) |
| `research::get_tsi` | SQLite `research_tsi` | ADR-146 TSI window (Blau 1991 True Strength Index = 100·EMA₁₃(EMA₂₅(ΔP))/EMA₁₃(EMA₂₅(|ΔP|)) — first double-smoothed momentum oscillator) |
| `research::get_garch11` | SQLite `research_garch11` | ADR-147 GARCH11 window (Bollerslev 1986 GARCH(1,1) conditional-variance fit via coordinate-descent grid MLE — first parametric volatility persistence model, shipping α/β/persistence/half-life/unconditional variance) |
| `research::get_sadf` | SQLite `research_sadf` | ADR-147 SADF window (Phillips-Wu-Yu 2011 Sup-ADF explosive-root / bubble test — first asymmetric tail-window stationarity test, complements ADF/KPSS/PPROOT) |
| `research::get_cordim` | SQLite `research_cordim` | ADR-147 CORDIM window (Grassberger-Procaccia 1983 correlation dimension D2 at m=3 embedding — first nonlinear-dynamics dimension surface, distinct from monofractal Hurst/DFA/Higuchi) |
| `research::get_skspec` | SQLite `research_skspec` | ADR-147 SKSPEC window (30-bar rolling skewness spectrum: mean/std/min/max/range — first skewness-stability diagnostic, complements RETQUANT full-window skew) |
| `research::get_automi` | SQLite `research_automi` | ADR-147 AUTOMI window (auto-mutual-information at lags 1/5/10 via k=8 equiprobable histogram binning — first information-theoretic ACF, catches nonlinear dependence invisible to classical ACF) |
| `research::get_ingested_articles` | SQLite `research_web_articles` | ADR-130 INGEST_RESEARCH window + packet Return Path footer (FIFO bag of web-search articles echoed back from AI agents, URL-deduped, timestamp-wins, capped at 50 per symbol) |
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
- ADR-108 / 109 / 110 / 111 / 112 / 113 / 114 / 115 / 116 / 117 / 118 / 119 / 120 / 121 / 122 / 123 / 124 / 125 / 126 / 127 / 128 / 129 / 131 / 132 / 133 / 134 / 135 / 136 / 137 / 138 / 139 / 140 / 141 / 142 / 143 / 144 / 145 / 146 / 147 — Godel parity research surfaces
- ADR-130 — Web-research ingest from AI agents + RESEARCH_PACKET viewer (tree-nav + scrollable text)
