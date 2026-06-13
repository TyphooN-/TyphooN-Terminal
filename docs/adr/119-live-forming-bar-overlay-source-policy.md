# ADR-119: Live Forming-Bar Overlay Source Policy

**Status:** Accepted
**Date:** 2026-06-13
**Related:** ADR-099 (Kraken WS OHLC), ADR-110 (market session status), ADR-112 (equities bar-sync lanes), ADR-113 (cross-source equity merge)

## Context

The native app had an old Alpaca trade/quote streaming path:

- `BrokerCmd::StartStream`
- `AlpacaBroker::start_stream`
- `BrokerMsg::StreamTick` / `BrokerMsg::StreamQuoteTick`
- `engine/src/core/bar_builder.rs`
- `native/src/app/app_runtime_stream_ticks.rs`

That path was no longer reachable from the GUI after broker-command cleanup. It was also the wrong abstraction for the current product direction:

- Kraken is the primary live market-data lane.
- xStocks/equities have provider/session quirks (24/5 sessions, delayed iapi quotes, weekend hard-close behavior) that must be explicit.
- The chart already has a live overlay path: `ChartState::apply_live_quote_update` records bid/ask freshness and folds the quote midpoint into the forming bar through `apply_forming_price_update`.
- Persisting synthetic quote bars from the egui thread previously froze the UI during cache contention; live overlays must stay in memory while closed bars are persisted by background workers.

## Decision

Delete the unreachable Alpaca `StartStream` / `StreamTick` / `StreamQuoteTick` trade-tick path and the unused `BarBuilder` module.

Live forming-bar UX is not owned by the deleted path. It is owned by provider-specific live quote and closed-bar lanes:

1. `BrokerMsg::Quote` → `handle_broker_quote` → `ChartState::apply_forming_price_update`.
2. `BrokerMsg::KrakenBookQuoteTick` → `handle_kraken_book_quote_tick` → `ChartState::apply_live_quote_update`.
3. `BrokerMsg::KrakenEquityQuote` → `handle_kraken_equity_quote` → `ChartState::apply_live_quote_update`.
4. `ChartState::apply_live_quote_update` stores live bid/ask, marks quote freshness/delay, and calls `apply_forming_price_update(mid)`.
5. `ChartState::fresh_live_quote_mid` gates stale/wide-spread quotes before watchlist, MTF Grid, positions, and chart calculations treat them as current price.

Closed Kraken WS OHLC bars remain a separate persistence lane under ADR-099. Open/in-progress values are an in-memory UI overlay until the closed bar is available.

## UX requirements

The removal of `StreamTick` / `StreamQuoteTick` must not regress chart richness:

- Live forming bars must continue to update while a valid live quote source is active.
- The magenta/active forming-bar visual remains a chart-rendering concern; it should be driven by the chart's current forming bar state, not by the historical existence of an Alpaca trade-tick enum.
- Extended-hours xStocks display must continue to use session-aware quote policy:
  - keep Friday's after-hours snapshot visible over the weekend where appropriate;
  - suppress stale bid/ask overlays during the hard weekend close;
  - prefer fresh realtime quotes over delayed iapi quotes during CORE;
  - do not let delayed/stale extended quotes overwrite fresher realtime state.
- If a provider lacks a live quote for a symbol/timeframe, the chart should fall back to cached bars instead of manufacturing per-trade bars from an unrelated provider.

## Future direction

If we need richer live forming bars than midpoint quote overlays, implement a single provider-neutral live bar overlay model, not a resurrected Alpaca-specific stream path.

That model should:

- accept provider-tagged quote/trade/open-bar updates;
- keep source, delay, session, and staleness metadata visible;
- update chart state in memory only;
- persist only confirmed closed bars through the existing background cache writers;
- include tests that prove CORE, PRE/AFTER/OVERNIGHT, weekend-close, and wide-spread stale quote behavior.

## Consequences

- Less dead command/message surface.
- No unreachable websocket task or dormant BarBuilder state in the native app.
- Live forming-bar behavior is easier to reason about because there is one chart state path (`apply_forming_price_update` / `apply_live_quote_update`) instead of a dead trade stream plus provider-specific live quote handlers.
- Any future enhancement must improve the provider-neutral overlay path and preserve the current extended-hours UX rather than reintroducing a hidden provider-specific path.
