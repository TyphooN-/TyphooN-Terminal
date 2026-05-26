# ADR-208: Xynth Feature Parity Target

**Status:** Reference audit, not an active implementation plan
**Date:** 2026-05-02

## Context

Xynth publicly positions itself as an AI market analysis agent with unified access to market data, options analytics, fundamentals, earnings, social/news search, visualizations, screening, and backtesting.

Public references used:

- Xynth home page: https://xynth.com/
- Xynth handbook overview: https://resources.xynth.finance/
- Data coverage: https://resources.xynth.finance/tools/data/
- Assets coverage: https://resources.xynth.finance/tools/assets/
- Market screener: https://resources.xynth.finance/tools/market-screener/
- Technical analysis: https://resources.xynth.finance/tools/technical-analysis/
- Fundamentals: https://resources.xynth.finance/tools/fundamentals/
- Earnings: https://resources.xynth.finance/tools/earnings/
- Unified search: https://resources.xynth.finance/tools/unified-search/
- Backtesting: https://resources.xynth.finance/tools/backtesting/
- Visuals: https://resources.xynth.finance/tools/visuals/
- Strategy re-run: https://resources.xynth.finance/tools/re-run

## Parity Matrix

| Xynth-documented capability | TyphooN status | Gap / condition to reopen |
|---|---:|---|
| Natural-language AI analyst over market data | Partial | TyphooN has AI sessions and research packets. Need planner/executor workflow that turns prompts into repeatable data pulls, calculations, charts, and summary reports. |
| Stocks, crypto, indices, forex live/historical data | Mostly covered | Stocks/crypto via Alpaca/Kraken/CryptoCompare/tastytrade; forex/indices via MT5/Darwinex where configured. Add direct forex/index fallback such as Polygon, Twelve Data, or Tiingo for users without MT5. |
| Intraday OHLC and multi-timeframe charts | Covered/exceeds | Native GPU charting, MTF grids, LAN cache, and multiple broker feeds exceed Xynth's documented chart basics. |
| Technical analysis indicators | Covered/exceeds | TyphooN research packet exposes a large TA surface and native chart indicators. Keep adding missing TA-Lib parity where needed. |
| Fundamentals and financial fields | Partial | Yahoo/FMP/fundamentals engine covers many fields. Xynth claims 5,000+ fields; we need normalized provider-backed field registry and peer/industry comparison. |
| Market screener with natural language | Partial | Screener framework exists. Need NL-to-field mapping, field registry, saved screens, and iterative refinement. |
| Earnings historical/future/surprise data | Partial | Earnings overlays and research tables exist. Need full projected/confirmed earnings query surface, surprise stats, expected move, and straddle P&L history. |
| Options chains, Greeks, IV, P&L diagrams | Partial/strong local analytics | TyphooN has options chain, Black-Scholes Greeks, IV surface, strategy/P&L tools. Need richer historical option OHLC, NBBO, OI history, and market-wide options aggregates. |
| Options flow, sweeps, floor/multileg, premium flows | Missing provider | Requires OPRA-derived flow feed or vendor API. Candidate sources: Unusual Whales, Tradier, Polygon options trades/quotes, ThetaData, ORATS, Intrinio, Cboe/OPRA via vendor. |
| Gamma exposure, vanna/charm, dealer positioning | Partial math, missing feed | Need per-contract OI/volume/IV snapshots and provider methodology. Candidate sources: ORATS, SpotGamma, SqueezeMetrics, Unusual Whales, Polygon/OPRA + internal model. |
| Dark pool/block trades | Partial | Current FINRA short-volume/dark-pool style view is not equivalent to real-time block/dark-pool prints. Need FINRA TRF/block trade feed or vendor such as Polygon, Unusual Whales, Intrinio, or Nasdaq Basic/TotalView-derived products. |
| Short borrow data | Missing provider | Need borrow fee/availability source. Candidate sources: Ortex, S3, Fintel, Interactive Brokers SLB, Nasdaq Data Link datasets. |
| SEC filings and insider activity | Covered/strong | SEC filings, FTS, Form 4 insider parsing, alerts, and LAN sync are present. |
| Politician/congress trades | Covered/partial | Congress trading window exists. Need ongoing source hardening and Senate/House coverage normalization. |
| Analyst ratings and price targets | Covered/partial | Finnhub ratings/targets exist. Add multi-provider consensus/history if needed. |
| News, Reddit, Twitter unified search | Partial | News exists. Need Reddit API and X API/Firehose alternatives, plus citation-preserving search/report workflow. Candidate sources: Reddit API, X API, GDELT, NewsAPI, Marketaux, Finnhub, Benzinga, Perplexity/Sonar. |
| Visualizations | Covered/exceeds in native app | Native charts, plots, heatmaps, options P&L, vol surface, correlation, portfolio/risk charts. Need prompt-driven chart generation for arbitrary query results. |
| Backtesting by natural language | Partial | Backtest engine, optimizer, walk-forward exist. Need prompt-to-strategy parser and saved strategy re-run workflow. |
| Strategy re-run and automatic summary | Missing product workflow | Need saved research plans that re-execute with fresh data, emit diff, summary, charts, and PDF/markdown export. |

## Required Data Sources To Meet Or Exceed Xynth

Minimum practical provider set:

1. **Options flow and historical option data:** Unusual Whales or Polygon/OPRA/ThetaData/ORATS.
2. **Gamma/dealer positioning:** ORATS/SpotGamma/SqueezeMetrics or internal model fed by complete OI/IV/volume data.
3. **Real dark-pool/block prints:** FINRA TRF/block trade vendor, Polygon, Intrinio, Nasdaq/CTA/UTP-derived provider.
4. **Short borrow:** Ortex/S3/Fintel/IBKR SLB/Nasdaq Data Link.
5. **Social sentiment:** Reddit API plus X API or a compliant social data vendor.
6. **Fundamental field breadth:** FMP + Financial Modeling Prep bulk, Intrinio, Tiingo, Nasdaq Data Link, or another normalized fundamentals vendor.
7. **Earnings/expected move:** Finnhub/FMP plus options-derived expected-move calculation from options chain snapshots.

## Decision

Do not clone Xynth as a web chatbot. TyphooN should meet/exceed the documented capability set by using its existing advantages:

- Native GPU charting and local-first cache.
- Broker execution and risk management.
- LAN server/client sync.
- Research packet and AI session persistence.
- Full data provenance in SQLite tables.

The missing work is mostly data acquisition and orchestration, not charting or core analytics.

## Reopen Criteria

This ADR should not be used as a standing task list. Reopen it only when a
concrete provider or product workflow is selected, then write a scoped ADR for
that slice. Valid reopen slices are:

1. Provider-neutral market-intelligence ingestion for a chosen paid/free data source.
2. Normalized SQLite tables for one acquired data family, with LAN-sync coverage.
3. Natural-language screening field registry tied to existing provider-backed fields.
4. `ResearchPlan` persistence and rerun reports over existing local data.
5. AI planner/executor workflow with auditable local tool calls and citations.

## Consequences

- **Pro:** Xynth parity is achievable without changing TyphooN's native/local-first architecture.
- **Pro:** TyphooN can exceed Xynth where it already has broker execution, local cache ownership, LAN sync, risk tooling, and native charting.
- **Con:** Full options-flow/GEX/dark-pool/borrow parity requires paid data feeds.
- **Con:** Natural-language workflows need careful guardrails so AI plans remain repeatable, auditable, and source-grounded.
