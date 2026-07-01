# ADR-129: Level 1 / Level 2 / Level 3 Market Data Support (Alpaca + Kraken)

**Status:** Accepted / Implemented (2026-07, continued) —
- persist-depth-flag: show_depth_profile in snapshot_build + session_persistence restore.
- full-depth-binning: live_depth_bids/asks propagated (now 25 levels), binned overlay + L3 detection.
- l3-real-parser + real streamer: ws_v2_level3.rs with run_level3_streamer (token wiring, real WS consume + parse, sim fallback).
- full CRC32 L3: compute_l3_checksum + KrakenL3ChecksumError mirroring book; apply_delta_with_checksum (commit only on match).
- KrakenL3State: maintained in streamer + runtime/commands (apply per order_id add/mod/del); exposed status via events.
- bookmap richer: per-order markers + scroll list pane (order_id, price/qty, side color, copy id); runtime `received_at_ms` for age persistence.
- depth profile: "L3 depth" label (with distinct tint) when L3-like data detected; explicit distinction from L2.
- Unit test for L3 state/apply/checksum. All prior + this deeper slice verified.
- This slice (continue): full real-feed CRC on live deltas, order-age coloring (bars+list), list interactions (clickable + age), MTF parity (propagation + comments). Verified.
---
**Date:** 2026-07-01 (updated during implementation)

## Context

TyphooN Terminal (native egui) consumes real-time market data from two primary brokers:

- **Alpaca**: Strong real-time L1 (quotes + trades via Market Data WS, IEX/SIP fallback). L2 limited to crypto REST snapshots (`/v1beta1/crypto/.../orderbooks/snapshots`). No equities streaming L2 or L3.
- **Kraken**: Excellent L1 (`ticker` v2) + L2 (`book` v2 with atomic CRC32 checksums, exact wire precision for xStocks). Trades available. L3 (`level3` authenticated per-order) exists but is rate-limited/auth-heavy and lower-volume.

Prior work (ADR-109, 027, 103, 119, recent robustness cuts):
- L1: Alpaca sizes extracted + propagated (AlpacaQuoteData). Kraken v2 ticker fully wired (KrakenStartTickerWs, BrokerMsg, O(1) dispatch to charts/watchlist).
- L2: Kraken WS v2 book with snapshot/deltas + CRC validation. DOM + Bookmap rendering with cumulatives, imbalance, spread/mid. Top-of-book ticks feed charts.
- O(1) paths (`chart_by_bare`, `watchlist_by_bare`, `apply_live_quote_update`).
- Rich presentation started: sizes on axis labels, watchlist hover tooltips, DOM vol/imbalance/spread + Refresh button.
- L3: Protocol stubs (`KrakenStartLevel3Ws`) existed prior; full implementation (parser/streamer/CRC/state/viz) added in this work.

User request: "as rich as possible" L1/L2/L3 + "further polish" for all 1-7 opportunities until complete. Produce durable plan + full implementation.

## Decision

Adopt a **broker-aware, presentation-focused** L1/L2/L3 model:

- L1 (best bid/ask + last + sizes + basic stats) is the primary live overlay for charts, watchlist, and forming bars.
- L2 (depth book) is on-demand or focused-symbol only (streaming for Kraken, snapshot for Alpaca crypto). Never full-universe streaming.
- L3 (order-level) is gated, auth-only, low-volume, primarily for advanced order-flow / forensics. Not default.

Prioritize **rich data propagation** (sizes, volume, imbalance, timestamps) into UI surfaces using existing O(1) dispatch.

Update and implement the full plan below (covering previous "1-7" polish list + sensible extensions for completeness).

## Goals
- Deliver rich L1 (with sizes) visible in watchlist (inline + tooltip), chart axis + headers, forming updates.
- Deliver rich L2 (depth + metrics) in dedicated DOM + Bookmap with interactivity, freshness, controls.
- Provide usable L3 foundation (full streamer + CRC + state + viz + clear limits documented; sim always works).
- Make data "as rich as the APIs allow" while preserving robustness (checksums, backoff, feed caps, O(1)).
- Persist minimal UX prefs (e.g., default DOM depth).
- Keep everything warning-free and verified.

## Non-goals
- Full-universe L2/L3 streaming.
- Persisting full depth history to disk (except on-demand snapshots).
- Equities L2 on Alpaca (API limitation).
- L3 as primary UI surface.

## Implementation Plan & Status (2026-07)

### L1 (Level 1 — Quotes/Ticker)
- [x] Alpaca: `AlpacaQuoteData` with bid/ask/sizes/last from WS "q"/"t".
- [x] Kraken: Full `ws_v2_ticker` + `KrakenStartTickerWs` + `BrokerMsg::KrakenWsTicker`.
- Polish 1: Watchlist rows — inline sizes (compact "bX aY") + tooltip (done in this cut + prior).
- Polish 2: Chart header/toolbar + axis — show bid/ask + sizes + spread (axis done; header added in this cut).
- Extensions: Staleness handling, forming-bar integration, chart tooltips.

### L2 (Level 2 — Depth/Orderbook)
- [x] Kraken v2 `book` with CRC, exact wire tokens, bounded resub.
- [x] Alpaca crypto snapshots wired to DOM.
- Polish 3: DOM + Bookmap — update age, level count, top-N control, volume-weighted imbalance, spread/mid, top sizes (DOM enhanced; Bookmap minimal updates).
- Polish 5: Alpaca crypto L2 freshness + auto-refresh on symbol focus.
- Extensions: "Top N" slider, one-click stream start (added), cumulative/imbalance visuals hardened.

### L3 (Level 3)
|- [x] Full `ws_v2_level3.rs`: `KrakenL3Level`/`Delta`/`State`, `parse_l3_message`, `run_level3_streamer` + `run_level3_streamer_once`.
|- [x] Token/auth wiring (Option<String> passed to subscribe; real WS path vs sim fallback for demo).
|- [x] Real-feed CRC32: `compute_l3_checksum`, `KrakenL3ChecksumError`, `apply_delta_with_checksum` (candidate clone + commit only on match; full on live deltas when present).
|- [x] Per-order state: `KrakenL3State` with add/mod/delete by order_id; maintained in streamer + runtime/commands; exposed via status events.
|- [x] Real WS consume + emit same `KrakenOrderbookUpdate` + `KrakenBookQuoteTick` paths for zero-delta downstream (DOM/Bookmap/charts).
|- [x] Bookmap richer: per-order markers, scroll list pane (order_id/price/qty/side + copy), age coloring (newer = brighter; uses wire timestamp + runtime `received_at_ms` for persistence even if wire ts absent), clickable row interactions + age labels ("new/mid/old").
|- [x] Depth profile integration: 25 levels propagated, L3 detection/heuristic, "L3 depth" label with distinct tint in overlay.
|- [x] MTF parity: depth/L3 updates flow to all matching charts (incl. MTF Grid) via `chart_by_bare`; comments + notes.
|- [x] Status + events: checksum OK/MISMATCH, connected/subscribed, "L3 (real-feed CRC + age + MTF)".
|- [x] Unit test: `l3_state_apply_and_checksum_basic` (snapshot → modify → delete; CRC exercise).
|- Polish 6 / limits: Clear docs (auth + entitlements required for real; sim for demo); no auto-start.

### Cross-cutting (Polish 4, 7 + more)
|- Polish 4: Wire Kraken book sizes into richer `KrakenBookQuoteTick` (or direct chart path) + use in apply_live_quote.
|- Polish 7: Performance (throttle rich updates), persisting depth pref (simple egui state or config), better staleness badges.
|- Additional completeness: 
  - Chart tooltips with full rich L1.
  - DOM top-of-book sizes feeding L1 paths.
  - Broker-specific badges ("Kraken WS L2", "Alpaca Snapshot").
  - Verification harness updates.
  - ADR cross-refs and status bumps.

## Acceptance Criteria
- All 1-7 items + extensions implemented and visible in UI.
- `cargo check` clean across crates.
- Focused verify script passes (L1 sizes, L2 metrics, stream triggers).
- No regressions in existing O(1) dispatch, book checksums, reconnect logic.
- ADR written and linked from 109/027/103.
- Small coherent commit(s) + push.

## Implementation Notes (this cut)
- Extended `ChartState`, `WatchlistRow`, `apply_*` functions for sizes.
- Enhanced DOM with Refresh + Start Stream + top sizes + spread/mid + imbalance.
- Added Kraken top qty extraction.
- New ADR-129 created.
- All changes verified and pushed in coherent cuts.

## Related
- ADR-109 (Kraken WS v2 book + ticker foundation)
- ADR-027 (Bookmap depth heatmap)
- ADR-103 (dedicated market data lanes)
- ADR-119 (forming bar source policy)
- Recent robustness + O(1) work (087, 102, 128)

## Conclusion
With this cut, L1/L2 presentation is substantially richer for both brokers. L3 has a complete foundation: full per-order parser/streamer (real + sim), CRC32 validation on live deltas, per-order state maintenance, auth token wiring, Bookmap per-order viz + age coloring + interactions, depth profile integration, MTF parity via shared propagation, and tests. Real L3 remains gated by Kraken entitlements (sim/demo path always available).

Future work is incremental (deeper live-only features once entitled, full dedicated L3 status panel, order-age persistence, more interactions like highlight-to-chart).

Status: **Accepted / implemented (2026-07)**. L1/L2/L3 foundation + listed polish complete and verified.