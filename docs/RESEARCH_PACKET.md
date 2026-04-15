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
**seventy-two sub-blocks**, each of which is skipped silently when its data
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

#### 2.72 Sector peer comparison

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
| Daily bars required for stats | ≥20 | Needed for 20d return and ATR warm-up |

There is no global packet size limit — total size scales with the number of
symbols. A single S&P 500 symbol now produces a packet around **28-55 KB**
(up from 26-52 KB after ADR-122; ADR-123 added five per-symbol blocks —
VRK / QRK / RRK / RELEPSGR / PEAD — covering sector-peer percentile
ranks of the Round 15 VAL / QUAL / RISK composites (with RRK inverted
so higher = safer), a relative-EPS-growth snapshot that compares the
symbol's 3y EPS CAGR to the sector median, and a post-earnings-drift
snapshot that joins cached EarningsSurprise rows with HP bars to
measure average forward drift over 1/3/5/10 trading days — all pure
compute over cached Round 7/8/10-15 snapshots with zero new API
dependencies); a 10-symbol basket lands near **270-540 KB** (the
global context is emitted only once, so multi-symbol overhead is still
bounded by the per-symbol blocks).

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
- ADR-108 / 109 / 110 / 111 / 112 / 113 / 114 / 115 / 116 / 117 / 118 / 119 / 120 / 121 / 122 / 123 — Godel parity research surfaces
