# ADR-129: Level 1 / Level 2 / Level 3 Market Data Support (Broker-Modular: Alpaca + Kraken now)

**Status:** Accepted / Implemented (2026-07, continued) —
- persist-depth-flag: show_depth_profile in snapshot_build + session_persistence restore.
- full-depth-binning: live_depth_bids/asks propagated (now 25 levels), binned overlay + L3 detection.
- l3-real-parser + real streamer: ws_v2_level3.rs with run_level3_streamer (token wiring, real WS consume + parse, sim fallback).
- full CRC32 L3: compute_l3_checksum + KrakenL3ChecksumError mirroring book; apply_delta_with_checksum (commit only on match).
- KrakenL3State: maintained in streamer + runtime/commands (apply per order_id add/mod/del); exposed status via events.
- bookmap richer: per-order markers + scroll list pane (order_id, price/qty, side color, copy id); runtime `received_at_ms` for age persistence.
- depth profile: "L3 depth" label (with distinct tint) when L3-like data detected; explicit distinction from L2.
- Unit test for L3 state/apply/checksum. All prior + this deeper slice verified.
- Continued polish: public-trade live execution markers on chart/right axis, watchlist L1 size freshness parity, Bookmap L3 selected-order persistence/header/heatmap highlight, shared DOM depth preference across stream entrypoints, L3-aware DOM metrics, and L3 entitlement/status surfacing.
- capability-model (code): the previously conceptual "each broker advertises L1/L2/L3 capabilities" is now a concrete typed model — `typhoon_engine::broker::capabilities` (`MarketDataSupport`, `DepthAssetScope`, `BrokerMarketDataCapabilities`, and `OrderBroker::{l1,l2,l3}_support` / `market_data_capabilities`). The native depth gate routes through it (`depth_stream_supported(broker, …)`) so gating is broker-parameterized (exhaustive match ⇒ adding a broker is a compile error until its depth behavior is declared). See "Broker Capability Model (code)" below.
- typed-provenance (code): the last hard-coded provenance strings are gone — `MarketDataProvenance` + `MarketDataTransport` (in `broker::capabilities`) are the single source of truth for payload `source`/`transport`. All producers (engine snapshots/WS, broker-runtime WS v2 L2/L3, native L3 sim) stamp it; the DOM badge consumer parses it back via `OrderBroker::from_persist_str` + `MarketDataTransport::from_wire`. Wire tokens unchanged; behavior identical.
- reconnect-robustness (code): all four Kraken WS market-data lanes (ticker/book/trade/level3) now share one self-healing reconnect discipline — capped exponential backoff and **no permanent give-up on a transient failure burst**. The public-trade tape previously *terminated for the session* after 5 consecutive failures (silently dropping live execution markers / forming-bar ticks); it now retries indefinitely with a one-shot "degraded" status, and the broker-runtime drains + surfaces its lifecycle events like the other lanes — which also fixes a latent unbounded event-channel buildup. Alpaca L1/trade WS already reconnect with backoff + 75 s staleness detection; L2 book checksum mismatch already keeps the last-good book and resubscribes a fresh snapshot.
---
**Date:** 2026-07-01 (updated during implementation)

## Context

TyphooN Terminal (native egui) currently consumes real-time market data from two primary brokers, but the L1/L2/L3 design is broker-modular rather than hard-coded to a single primary broker:

- **Alpaca**: Strong real-time L1 (quotes + trades via Market Data WS, IEX/SIP fallback). L2 limited to crypto REST snapshots (`/v1beta1/crypto/.../orderbooks/snapshots`). No equities streaming L2 or L3.
- **Kraken**: Excellent L1 (`ticker` v2) + L2 (`book` v2 with atomic CRC32 checksums, exact wire precision for xStocks). Trades available. L3 (`level3` authenticated per-order) exists but is rate-limited/auth-heavy and lower-volume.

Future broker modules must plug into the same capability model instead of forking UI semantics: likely next is restoring **tastytrade** after the Alpaca/Kraken combover is complete, with **Binance** a plausible later crypto venue. Each broker advertises supported L1/L2/L3 capabilities, entitlement constraints, freshness semantics, and snapshot-vs-stream behavior; UI surfaces consume normalized capabilities and degrade cleanly when a broker lacks a level.

Prior work (ADR-109, 027, 103, 119, recent robustness cuts):
- L1: Alpaca sizes extracted + propagated (AlpacaQuoteData). Kraken v2 ticker fully wired (KrakenStartTickerWs, BrokerMsg, O(1) dispatch to charts/watchlist).
- L2: Kraken WS v2 book with snapshot/deltas + CRC validation. DOM + Bookmap rendering with cumulatives, imbalance, spread/mid. Top-of-book ticks feed charts.
- O(1) paths (`chart_by_bare`, `watchlist_by_bare`, `apply_live_quote_update`).
- Rich presentation is implemented across chart axis/right-axis execution tags, watchlist inline/tooltip sizes with freshness parity, DOM vol/imbalance/spread/top sizes, and Bookmap L2/L3 interactions.
- L3: `KrakenStartLevel3Ws` is now a gated real/sim implementation path with parser/streamer/CRC/state/viz, entitlement status, and Bookmap/DOM/depth integration.

User request: "as rich as possible" L1/L2/L3 + "further polish" for all 1-7 opportunities until complete. Produce durable plan + full implementation.

## Decision

Adopt a **broker-modular, capability-aware, presentation-focused** L1/L2/L3 model:

- L1 (best bid/ask + last + sizes + basic stats) is the primary live overlay for charts, watchlist, and forming bars, regardless of selected primary broker.
- L2 (depth book) is on-demand or focused-symbol only (streaming for Kraken, snapshot for Alpaca crypto today). Never full-universe streaming. Future brokers must mark L2 as stream, snapshot, delayed, or unsupported.
- L3 (order-level) is gated, auth/entitlement-heavy, low-volume, primarily for advanced order-flow / forensics. Not default. Future brokers with L3-like feeds must expose explicit entitlement/status/fallback behavior before UI enables real starts.

Prioritize **rich data propagation** (sizes, volume, imbalance, timestamps) into UI surfaces using existing O(1) dispatch.

Update and implement the full plan below (covering previous "1-7" polish list + sensible extensions for completeness).

## Goals
- Deliver rich L1 (with sizes) visible in watchlist (inline + tooltip), chart axis + headers, forming updates.
- Deliver rich L2 (depth + metrics) in dedicated DOM + Bookmap with interactivity, freshness, controls.
- Provide usable L3 foundation (full streamer + CRC + state + viz + clear limits documented; sim always works).
- Make data "as rich as the APIs allow" while preserving robustness (checksums, backoff, feed caps, O(1)) and broker modularity.
- Persist minimal UX prefs (e.g., default DOM depth).
- Keep everything warning-free and verified.

## Non-goals
- Full-universe L2/L3 streaming.
- Persisting full depth history to disk (except on-demand snapshots).
- Equities L2 on Alpaca (API limitation).
- Broker-specific UI forks for each future provider; use normalized L1/L2/L3 capability/status models instead.
- L3 as primary UI surface.

## Implementation Plan & Status (2026-07)

### L1 (Level 1 — Quotes/Ticker)
- [x] Alpaca: `AlpacaQuoteData` with bid/ask/sizes/last from WS "q"/"t".
- [x] Kraken: Full `ws_v2_ticker` + `KrakenStartTickerWs` + `BrokerMsg::KrakenWsTicker`.
- [x] Polish 1: Watchlist rows — inline sizes (compact "bX aY") + tooltip, using the same 30s freshness rule as chart overlays.
- [x] Polish 2: Chart axis/right-axis — bid/ask labels, executed live-trade line/tag, and forming-bar/live-trade integration.
- [x] Extensions: staleness handling, forming-bar integration, chart/live-trade tooltip cues.
- [x] Reconnect parity (robustness): the Kraken ticker/book/trade/level3 WS lanes share one self-healing reconnect (capped exponential backoff via `compute_*_reconnect_backoff`, terminal only when the consumer drops the channel — never on a transient failure burst). The public-trade tape (executed markers + M1/M5 forming-bar freshness) no longer dies for the session after 5 consecutive failures; the broker-runtime now drains and surfaces its lifecycle events as `KrakenWsStatus` like the ticker/book/L3 lanes.

### L2 (Level 2 — Depth/Orderbook)
- [x] Kraken v2 `book` with CRC, exact wire tokens, bounded resub.
- [x] Alpaca crypto snapshots wired to DOM.
- [x] Polish 3: DOM + Bookmap — update age, level count, top-N control, volume-weighted imbalance, spread/mid, top sizes, hover tooltips, density scaling + explicit top-% dense warning in Bookmap L2 header, provider/status badges.
- [x] Polish 5: Alpaca crypto L2 snapshots remain snapshot-scoped and Order Flow/DOM snapshot fetch controls stay available; Kraken streaming L2 is focused/on-demand and spot-pair gated only.
- [x] Extensions completed: shared DOM depth slider/preference used by toolbar L2, Order Flow Stream L2, Bookmap Stream Depth, and Orderbook DOM Apply/Start Stream; toolbar L2 uses the same loaded Kraken-pair support guard as the floating windows; cumulative/imbalance visuals hardened; DOM metrics read L3 `limit_price`/`order_qty` as well as L2 `price`/`size`.

### L3 (Level 3)
- [x] Full `ws_v2_level3.rs`: `KrakenL3Level`/`Delta`/`State`, `parse_l3_message`, `run_level3_streamer` + `run_level3_streamer_once`.
- [x] Token/auth wiring (Option<String> passed to subscribe; real WS path vs sim fallback for demo).
- [x] Real-feed CRC32: `compute_l3_checksum`, `KrakenL3ChecksumError`, `apply_delta_with_checksum` (candidate clone + commit only on match; full on live deltas when present).
- [x] Per-order state: `KrakenL3State` with add/mod/delete by order_id; maintained in streamer + runtime/commands; exposed via status events.
- [x] Real WS consume + emit same `KrakenOrderbookUpdate` + `KrakenBookQuoteTick` paths for zero-delta downstream (DOM/Bookmap/charts).
- [x] Bookmap richer: per-order bid+ask markers, selected-order persistence on the Bookmap window, selected side/price/quantity in header with clear action, heatmap marker highlight/ring, scroll list pane (order_id/price/qty/side + copy), age coloring (newer = brighter; uses wire timestamp + runtime `received_at_ms` for persistence even if wire ts absent), clickable row interactions + age labels ("new/mid/old"), real/Demo L3 starts auto-opening the matching Bookmap window, and Bookmap window badges switching from L2 to L3 when rendering L3 payloads.
- [x] Depth profile integration: 25 levels propagated, L3 detection/heuristic, "L3 depth" label with distinct tint in overlay.
- [x] MTF parity: depth/L3 updates flow to all matching charts (incl. MTF Grid) via `chart_by_bare`; comments + notes.
- [x] Status + events: checksum OK/MISMATCH, connected/subscribed, "L3 (real-feed CRC + age + MTF)", explicit no-token/auth-entitlement messages, and native `kraken_l3_status` tracking from broker OrderResult messages.
- [x] Unit test: `l3_state_apply_and_checksum_basic` (snapshot → modify → delete; CRC exercise).
- Polish 6 / limits: Clear docs + UI status (auth + entitlements required for real; sim for demo); no auto-start.

### Cross-cutting (Polish 4, 7 + more)
- [x] Polish 4: Kraken book sizes flow through `KrakenBookQuoteTick`/live quote paths and are rendered under freshness guards.
- [x] Polish 7: focused/on-demand stream scope, O(1) dispatch, session `dom_depth` preference, every live depth stream entrypoint (including chart auto-L2 top-of-book) guarded to supported Kraken pairs, bounded checksummed book/L3 state, and explicit staleness/status labels.
- Additional completeness / future TODOs: 
  - [x] Broker-modular capability discipline documented for Alpaca/Kraken now, tastytrade restoration next-likely, and Binance possible later.
  - [x] Chart tooltips with full rich L1.
  - [x] DOM top-of-book sizes feeding L1 paths.
  - [x] Broker-specific DOM badges ("Kraken WS L2", "Kraken WS L3", "Kraken L3 demo", "Kraken snapshot", "Alpaca snapshot").
  - [x] Snapshot/stream payloads carry normalized `source`/`transport` metadata so DOM badges do not rely on symbol suffix heuristics.
  - [x] Legacy Kraken public-book WS status/update/error payloads use `source=kraken`, `transport=websocket` like v2 L2/L3.
  - [x] Bookmap and DOM L3 labels share one detector for explicit `is_l3` flags or per-order `order_id` fields on bids/asks.
  - Verification harness updates as each cut lands.
  - ADR cross-refs and status bumps as implementation changes.

## Broker Capability Model (code)

The "each broker advertises supported L1/L2/L3 capabilities" principle is a
concrete, typed model in the engine so every UI surface consumes normalized
capabilities instead of hard-coding a broker. **Any surface deciding whether to
offer L1/L2/L3 for a symbol must consult this model, not a broker identity
check.** This is the durable form of "we are modular — remember it in all
aspects of terminal design."

`typhoon-engine/src/broker/capabilities.rs`:

- `MarketDataSupport` — `Unsupported < Delayed < Snapshot < Stream` (ordered).
  Predicates: `is_available()` (any data), `is_realtime()` (snapshot|stream),
  `is_live()` (stream only — gates "Start Stream" affordances). `label()` for
  provenance/badges.
- `DepthAssetScope` — `None | CryptoOnly | SpotAndXStock | All`; which asset
  classes a broker's L2/L3 book covers, so depth is never offered on symbols a
  broker cannot serve.
- `BrokerMarketDataCapabilities` — `{ broker, l1, l2, l3, l2_scope, l3_scope,
  l3_entitlement_gated, notes }`.
- `OrderBroker::l1_support() / l2_support() / l3_support()` and
  `OrderBroker::market_data_capabilities()` — the single source of truth. Every
  arm is an **exhaustive match on `OrderBroker`**, so adding a broker fails to
  compile until its L1/L2/L3 support + depth scopes are declared here in one
  place.

Capability matrix (current + planned):

| Broker | L1 | L2 | L2 scope | L3 | L3 gated | Notes |
|---|---|---|---|---|---|---|
| **Alpaca** | Stream | Snapshot | crypto only | — | — | L1 WS (SIP/IEX); L2 crypto REST snapshots; no L3 |
| **Kraken** | Stream | Stream | spot + xStock | Stream | yes | book v2 CRC32; L3 auth/entitlement-gated (sim demo otherwise) |
| _tastytrade_ (planned) | Stream | Stream (entitled) | equities/futures | — | — | restore after Alpaca/Kraken combover (DXLink) |
| _Binance_ (planned) | Stream | Stream | crypto | Stream-like | no | plausible later crypto venue (diff-depth + trade stream) |

Native gating routes through this model. `common.rs::depth_stream_supported(
broker, symbol, kraken_pairs)` first checks `broker.l2_support().is_live()`
(snapshot-only/unsupported brokers short-circuit to `false`), then dispatches
per broker (Kraken → spot/xStock pair check; others declare their own arm).
`kraken_depth_stream_supported` is now a thin alias for the Kraken arm, so the
21 DOM/Bookmap/toolbar call sites and their tests are unchanged while the gate
is broker-parameterized. Depth is **symbol-routed, not primary-broker-routed**
(matches `handle_orderbook`, which serves depth from whichever broker can serve
the symbol) — so selecting a different Primary broker never disables depth that
another enabled broker can still stream.

L3 detection stays broker-agnostic (`orderbook_value_is_l3`: explicit `is_l3`
flag or per-order `order_id`), and snapshot/stream payloads carry **typed**
provenance rather than raw string literals: `MarketDataProvenance` +
`MarketDataTransport` (`websocket` | `snapshot` | `demo`) in
`broker::capabilities` are the single source of truth for the `source` /
`transport` wire vocabulary. `source` reuses `OrderBroker::as_persist_str`, so
every producer (engine snapshots/WS, broker-runtime WS v2 L2/L3, native L3 sim)
stamps it via `MarketDataProvenance`, and the DOM badge consumer
(`orderbook_provider_badge`) parses it back through
`OrderBroker::from_persist_str` + `MarketDataTransport::from_wire` — producer and
consumer share one typed vocabulary, with legacy heuristics kept only as
fallbacks for unstamped payloads.

## Acceptance Criteria
- All 1-7 items + extensions implemented and visible in UI.
- Broker capability model is typed, exhaustive over `OrderBroker`, and unit-tested; native depth gating routes through it with unchanged Alpaca/Kraken behavior.
- `cargo check` clean across crates.
- Focused verify script passes (L1 sizes, L2 metrics, stream triggers).
- No regressions in existing O(1) dispatch, book checksums, reconnect logic.
- ADR written and linked from 109/027/103.
- Small coherent commit(s) + push.

## Implementation Notes (this cut)
- Extended `ChartState`, `WatchlistRow`, `apply_*` functions for sizes.
- Enhanced DOM with Fetch L2 Snapshot + Start Stream + top sizes + spread/mid + imbalance.
- Added Kraken top qty extraction.
- New ADR-129 created.
- All changes verified and pushed in coherent cuts.

## Related
- ADR-109 (Kraken WS v2 L1/L2/trade/L3 market-data coverage)
- ADR-027 (Bookmap depth heatmap)
- ADR-103 (dedicated market data lanes)
- ADR-119 (forming bar source policy)
- Recent robustness + O(1) work (087, 102, 128)

## Conclusion
With this cut, L1/L2 presentation is substantially richer for both brokers. L3 has a complete foundation: full per-order parser/streamer (real + sim), CRC32 validation on live deltas, per-order state maintenance, auth token wiring, Bookmap per-order viz + age coloring + interactions, depth profile integration, MTF parity via shared propagation, and tests. Real L3 remains gated by Kraken entitlements (sim/demo path always available).

Future work is incremental: deeper live-only features once entitled, a dedicated L3 status/budget panel if real L3 becomes routinely used, retained depth-history/ring-buffer heatmap only after feed entitlement + texture budget exist, optional click-to-chart/focus affordances from selected L3 orders, and broker-module expansion after the Alpaca/Kraken combover. tastytrade is the likely next restored provider; Binance is a plausible later venue. Both must enter through the same capability/status/freshness model rather than special-casing UI behavior.

Status: **Accepted / implemented (2026-07)**. L1/L2/L3 foundation + listed polish complete and verified.
