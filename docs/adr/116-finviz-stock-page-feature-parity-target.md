# ADR-116: Finviz Stock-Page Feature Parity Target

**Status:** Reference audit + gap-closure plan (not a committed implementation schedule)
**Date:** 2026-06-12
**Related:** ADR-034 (fundamentals engine), ADR-056 (screener framework), ADR-073 (SEC filing DB), ADR-078 (multi-source news ingest), ADR-079 (research packet — research and indicator parity), ADR-092 (Xynth parity target), ADR-117 (StockTwits social-sentiment ingest)

## Context

Goal: present **100% of what a Finviz stock page shows** (reference: `https://finviz.com/stock?t=WOK&p=d`, captured 2026-06-12) inside the terminal / research packet, then exceed it with surfaces Finviz lacks (social sentiment via ADR-117, deep research/indicator surfaces via ADR-079, native risk analytics).

A Finviz stock page is four things: (1) a dense ~100-field fundamentals snapshot table, (2) a price chart with SMA overlays, (3) an insider-trading table, (4) an aggregated news/headlines list. Finviz site-wide additionally offers a 70+ filter **screener**, sector/industry **groups** (performance), and a heatmap **map**.

The finding below is that TyphooN already covers the large majority of the snapshot via `typhoon-engine/src/core/fundamentals.rs` (ADR-034), the research packet (ADR-079), SEC/insider (ADR-073), analyst ratings (Finnhub), and bar-derived technicals. "100%" is therefore mostly a **derivation + presentation** exercise (compute the few missing ratios/return-windows from data we already hold) plus a short list of true provider gaps.

## Finviz stock-page field inventory (2026-06-12)

- **Valuation:** P/E, Forward P/E, PEG, P/S, P/B, P/C, P/FCF, EV/EBITDA, EV/Sales
- **Per-share / EPS:** EPS (ttm), EPS next Y, EPS next Q, EPS this Y, EPS next 5Y, EPS past 3/5Y, Book/sh, Cash/sh
- **Profitability:** Gross Margin, Operating Margin, Profit Margin, ROA, ROE, ROIC
- **Performance:** Perf Week, Month, Quarter, Half Y, YTD, Year, 3Y, 5Y, 10Y
- **Ownership / shares:** Insider Own, Insider Trans, Inst Own, Inst Trans, Shs Outstand, Shs Float, Short Float, Short Ratio, Short Interest
- **Technicals:** RSI (14), ATR (14), SMA20, SMA50, SMA200, Volatility, Beta, Rel Volume, Avg Volume, 52W High, 52W Low, Prev Close, Change
- **Company / financials:** Market Cap, Enterprise Value, Income, Sales, Quick Ratio, Current Ratio, Debt/Eq, LT Debt/Eq, Payout, Dividend Est., Dividend TTM, Dividend Gr. 3/5Y, Dividend Ex-Date, Earnings (date), EPS/Sales Surpr., Sales Y/Y TTM, Sales Q/Q, EPS Y/Y TTM, EPS Q/Q, Sales past 3/5Y, Employees, IPO, Target Price, Recom, Optionable, Shortable
- **Insider table:** Relationship, Date, Transaction (Buy/Sale), Cost, #Shares, Value ($), #Shares Total, SEC Form 4 link
- **News table:** Date, Headline, Source, intraday performance impact %
- **Chart:** daily/weekly/monthly candles + SMA overlays + volume

## Parity matrix (stock page)

Legend: **Have** = present today · **Derive** = computable from data already held, needs wiring/exposure · **Gap** = needs a provider/field not currently ingested.

| Finviz field group | Status | Where it lives / what's missing |
|---|---|---|
| P/E, Fwd P/E, PEG, P/S, P/B, EV/EBITDA | **Have** | `fundamentals.rs` (`pe_ratio`, `forward_pe`, `peg_ratio`, `price_to_sales`, `price_to_book`, `ev_to_ebitda`) |
| P/C, P/FCF, EV/Sales | **Derive** | Have cash, FCF (`QuarterlyFinancial.free_cash_flow`), sales, EV → compute ratios |
| EPS (ttm), Book/sh, Cash/sh | **Derive/Have** | EPS in `QuarterlyFinancial.eps`; book/cash per share = book value / cash ÷ shares_outstanding |
| EPS next Y/Q/5Y, EPS this Y, EPS past 3/5Y | **Partial** | Forward estimates via FMP/Alpha Vantage research fetchers; past-growth derivable from quarterly history |
| Gross/Operating/Profit Margin, ROA, ROE | **Have** | `fundamentals.rs` (`gross`→`QuarterlyFinancial.gross_profit`, `operating_margin`, `profit_margin`, `roa`, `roe`) |
| ROIC | **Derive** | NOPAT / invested capital from financials we hold |
| Perf Week/Month/Quarter/Half/YTD/Year/3Y/5Y/10Y | **Derive** | `return_1m` exists; full window set computable from cached D1/W1 bars (research return surfaces, ADR-079) |
| Insider Own / Inst Own / Insider Trans / Inst Trans | **Have/Partial** | Insider activity (Form 4, ADR-073) + `InstitutionalHolder`; net-transaction % needs aggregation |
| Shs Outstand, Shs Float, Short Float, Short Ratio | **Have** | `shares_outstanding`, `float_shares`, `short_percent_of_float`, `short_ratio` |
| Short Interest (absolute), short-interest history | **Partial** | Short-interest history storage exists (research v92); absolute SI sourcing varies by provider |
| RSI(14), ATR(14), SMA20/50/200, Volatility, Beta, Rel/Avg Volume | **Have** | Native indicators + bar-derived stats; `beta` in `fundamentals.rs` |
| 52W High/Low, Prev Close, Change | **Have** | Bar cache |
| Market Cap, EV, Income, Sales | **Have** | `fundamentals.rs` (`market_cap`, `enterprise_value`) + `QuarterlyFinancial` |
| Quick/Current Ratio, LT Debt/Eq, Payout | **Derive/Partial** | Debt/Eq present (`debt_to_equity`); current/quick/LT-split/payout need balance-sheet line items |
| Dividend Est/TTM/Gr 3-5Y/Ex-Date | **Partial** | `dividend_yield` + dividend-snapshot storage; growth + ex-date precision partial |
| Earnings date, EPS/Sales Surprise | **Have** | Earnings + surprise surfaces (research, ADR-079) |
| Sales/EPS Y/Y & Q/Q, Sales past 3/5Y | **Derive** | From `QuarterlyFinancial` history |
| Employees | **Gap** | Not ingested; available from FMP/Yahoo profile |
| IPO date | **Have** | `ipo_date` |
| Target Price, Recom | **Have** | Finnhub analyst ratings/targets |
| Optionable / Shortable flags | **Gap** | Minor boolean flags; derive from broker asset metadata / options presence |
| Insider table (full columns) | **Have** | Form 4 parse (ADR-073) — relationship, date, transaction, shares, value, link |
| News table + source + impact % | **Have/Partial** | Multi-source news (ADR-078); intraday impact % is derivable from bars at headline timestamp |
| Chart + SMA overlays | **Have/exceeds** | Native GPU chart, MTF grid, full indicator suite |

## Beyond the stock page (site-wide Finviz)

| Finviz site feature | Status | Note |
|---|---|---|
| Screener (70+ descriptive/fundamental/technical filters) | **Partial** | Screener framework exists (ADR-056, `screener.rs`); needs a finviz-style filter registry + saved screens to reach 70+ filters |
| Groups (sector/industry performance) | **Partial/Derive** | Sector data in fundamentals; need a sector/industry aggregation + performance view |
| Maps (heatmap) | **Gap/UX** | No treemap heatmap window yet; native renderer can draw it from market-cap + perf |
| Insider feed (market-wide latest) | **Have** | SEC Form 4 scanner (ADR-073) |
| News / blogs aggregation | **Have** | ADR-078 multi-source ingest |

## Decision

1. **Treat the finviz stock-page field set as a coverage checklist** for the research packet and a future "Snapshot" window. Track it as the matrix above.
2. **Close the `Derive` gaps first** — they need no new provider, only computation + exposure from data already cached (perf-window returns, P/C, P/FCF, EV/Sales, ROIC, per-share book/cash, Y/Y & Q/Q growth, current/quick ratio where line items exist). These are the cheapest route to ">90% parity".
3. **Close `Gap` items opportunistically** from sources already wired (employees + optionable/shortable from FMP/Yahoo profile + broker asset metadata; dividend ex-date/growth precision; absolute short interest).
4. **Exceed finviz** with social sentiment (ADR-117), the deep TA-Lib/research surfaces (ADR-079), and native risk analytics finviz does not offer.
5. **All derived fields obey ADR-098 O(1) hot-path discipline** — compute on snapshot/refresh into a cached struct, never per-frame.

## Gap-closure TODOs (future)

- [ ] `FinvizSnapshot` aggregate struct + research-packet section `### Finviz-Style Snapshot` consolidating the matrix into one table.
- [ ] Derive perf-window returns (W/M/Q/H/YTD/Y/3Y/5Y/10Y) from cached bars into the snapshot.
- [ ] Derive P/C, P/FCF, EV/Sales, ROIC, Book/sh, Cash/sh, current/quick ratio, payout from held financials.
- [ ] Derive Sales/EPS Y/Y & Q/Q & past-3/5Y growth from `QuarterlyFinancial` history.
- [ ] Add Employees + Optionable/Shortable flags from FMP/Yahoo profile + broker asset metadata.
- [ ] Headline intraday impact % (price move at headline timestamp) for the news section.
- [ ] Optional finviz-style **treemap heatmap** window (market-cap × perf) and **sector/industry groups** view.
- [ ] Grow the screener (ADR-056) toward finviz's 70+ filter registry with saved screens.

## Consequences

- The honest baseline is **~85–90% of the finviz stock page already present**; reaching 100% is mostly derivation + a consolidated snapshot surface, not new ingestion.
- A single `FinvizSnapshot` table makes parity measurable and demoable, and becomes the natural home for the `Derive` fields.
- This ADR is a **reference audit**, not a delivery schedule; slices land as normal feature work and update the matrix.
