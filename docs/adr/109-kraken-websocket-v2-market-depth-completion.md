# ADR-109: Kraken WebSocket v2 Market Depth Completion

Status: Accepted / L2 book + checksum + L3 foundation implemented (updated 2026-07)
Date: 2026-06-06

## Context

TyphooN currently uses Kraken WebSocket v2, but not comprehensively.

Source audit:

- `typhoon-engine/src/broker/kraken/ohlc_ws.rs`
  - uses `wss://ws.kraken.com/v2`
  - supports public `ohlc`
  - builds v2 subscribe/unsubscribe frames
  - parses snapshot/update OHLC frames
  - runs one reconnecting streamer per interval
- `typhoon-engine/src/broker/kraken/mod.rs`
  - uses v2 `instrument` snapshot for Kraken tokenized equity/xStocks universe discovery
- `typhoon-engine/src/broker/kraken/public_book.rs`
  - still uses public WebSocket v1 endpoint `wss://ws.kraken.com`
  - subscribes to v1 `book` / `book-N`
  - parses v1 array frames with `as`/`bs` snapshot keys and `a`/`b` delta keys
- `typhoon-engine/src/broker/kraken/ws_v2.rs`
  - shared v2 endpoints, request ids, frame builders, ACK parsing, numeric/timestamp helpers
- `typhoon-engine/src/broker/kraken/ws_v2_ticker.rs`
  - v2 `ticker` parser, subscribe batching, reconnecting public streamer, and native focused-symbol L1 propagation are implemented
- `typhoon-engine/src/broker/kraken/ws_v2_book.rs`
  - v2 `book` parser/state helper, subscribe batching, reconnecting public streamer, native DOM/Bookmap/top-of-book quote wiring, and shared depth controls are implemented
  - atomic CRC32 checksum (candidate apply + commit only on match) + xStock wire precision implemented (2026-07)
- `typhoon-engine/src/broker/kraken/private_ws.rs`
  - private account feed is v1 shape: `ownTrades` and `openOrders`
  - no v2 `executions` or `balances` channel parser yet
- `typhoon-native/src/app/app_runtime_kraken_ws.rs` and broker message handlers
  - focused Kraken L1/L2/trade/L3 market-data dispatch; L2/L3 status propagation; low-timeframe WS freshness hints; no full-universe L2/L3

Kraken WebSocket v2 documentation exposes more than we currently consume:

- Level 1 public market data: `ticker` on `wss://ws.kraken.com/v2`
- Level 2 public market data: `book` on `wss://ws.kraken.com/v2`
- Level 3 authenticated market data: `level3` on `wss://ws-l3.kraken.com/v2`
- Trades: `trade` on `wss://ws.kraken.com/v2`
- Candles: `ohlc` on `wss://ws.kraken.com/v2` — already implemented
- Instruments: `instrument` on `wss://ws.kraken.com/v2` — snapshot path already implemented
- Authenticated account streams: `balances` and `executions` on `wss://ws-auth.kraken.com/v2`

Conclusion: substantial v2 support is now in place (OHLC full, instrument snapshot, ticker L1, public trades, book L2 with atomic checksum + robustness, and gated L3 foundation/status). Private account streams remain largely v1. Full v2 migration for balances/executions is still phased (see phases).

## Decision

Build a shared Kraken WebSocket v2 protocol layer and migrate market-data/account streams to v2 in bounded phases.

Do not attempt one giant "support everything" patch. That would create a brittle protocol blob and likely regress the current full-universe OHLC sync. The implementation should be channel-specific modules sharing a small common connection/subscription/parser framework.

## Goals

1. Support L1 ticker data for focused/active Kraken symbols.
2. Replace v1 public order-book plumbing with v2 L2 `book` support.
3. Add authenticated v2 L3 `level3` support with subscription budgeting and explicit UI gating.
4. Add v2 `trade` stream as the tick/time-and-sales feed for active symbols.
5. Migrate private v1 `ownTrades`/`openOrders` to v2 `executions`, and add `balances`.
6. Keep the existing OHLC full-universe streamer stable and warning-free.
7. Keep compile/manageability sane by splitting protocol modules by channel.

## Non-goals

- Do not stream full-universe L2/L3. That is wasteful and probably rate/connection hostile.
- Do not persist every tick/order-book update to the bar cache.
- Do not put L3 behind default auto-start. L3 is authenticated, rate-limited, depth-sensitive, and symbol-count limited.
- Do not mix v1 and v2 book state in the same parser. Migrate by adding v2 beside v1, then remove/retire v1 once verified.

## Proposed module layout

Under `typhoon-engine/src/broker/kraken/`:

- `ws_v2.rs`
  - shared constants/endpoints
  - request id generator
  - generic subscribe/unsubscribe frame builders
  - common ACK/error/status parsing
  - reconnect/backoff helpers
  - heartbeat/ping handling
- `ws_v2_ticker.rs`
  - `KrakenWsTicker`
  - subscribe frames for channel `ticker`
  - snapshot/update parser
- `ws_v2_book.rs`
  - `KrakenWsBookLevel`
  - `KrakenWsBookDelta`
  - `KrakenWsBookState`
  - subscribe frames for channel `book` with depth
  - v2 snapshot/update parser
  - checksum validation for top 10 bids/asks
- `ws_v2_trade.rs`
  - `KrakenWsTrade`
  - snapshot/update parser for recent/live trades
- `ws_v2_level3.rs`
  - authenticated endpoint `wss://ws-l3.kraken.com/v2`
  - `KrakenWsL3Order`, `KrakenWsL3Delta`, `KrakenWsL3BookState`
  - depth-aware subscription cost budgeting
  - checksum validation
- `ws_v2_account.rs`
  - authenticated endpoint `wss://ws-auth.kraken.com/v2`
  - `KrakenWsExecution`, `KrakenWsBalance`
  - replaces/augments v1 `private_ws.rs`

Native side:

- `typhoon-native/src/app/kraken_market_ws.rs`
  - app-level supervisor for active/focused symbol L1/L2/trade streams
  - symbol subscription reconciliation when chart/watchlist/MTF focus changes
  - coalesced UI events so egui is not spammed per tick
- `typhoon-native/src/app/kraken_l3_ws.rs`
  - opt-in L3 supervisor
  - only for selected symbols/depth
  - clear status/errors/rate-budget display

## Coverage matrix

| Kraken WS v2 channel | Current state | Target state | Priority |
| --- | --- | --- | --- |
| `ohlc` | Implemented and full-universe streamed | Keep; share future common v2 helpers only if low-risk | Done / preserve |
| `instrument` | One-shot snapshot implemented | Keep; optionally move parsing to channel module | Done / cleanup |
| `ticker` L1 | Parser + reconnecting public streamer implemented; native active/focused-symbol wiring feeds chart/watchlist/forming bars and size-aware UI | Keep focused/O(1), preserve freshness guards and no cache writes from egui | Done |
| `book` L2 | Parser + reconnecting public streamer + atomic CRC32 checksum (candidate-state apply, commit **only** on match) + exact wire-token precision preservation for xStocks (METAx etc.) + bounded resub (10) + exp backoff + ping/pong + unsubscribe frames implemented; v1 `public_book.rs` coexists for compat/legacy paths | Native L2 (KrakenStartOrderbookWs + orderbook window + top-of-book quote ticks) wired (uses v2); O(1) bare-symbol quote dispatch for KrakenBookQuoteTick in native; shared `dom_depth` across all stream entrypoints; full v1 retirement when verified | Done (core + robustness + UI depth pref 2026-07) |
| `trade` | Public trade stream wired into live execution price/volume/side, forming bar updates, right-axis execution tag, and M1/M5 freshness hints | Time-and-sales tape UI remains optional; preserve non-cache tick policy | Mostly done / UI tape optional |
| `level3` L3 | Foundation (ws_v2_level3.rs + streamer + CRC + per-order KrakenL3State + token + status + Bookmap/depth integration); broker emits explicit token/no-token entitlement messages and native tracks `kraken_l3_status` | Opt-in only; full budget panel and retained history wait for real entitlement/use | Foundation done / gated |
| `balances` | Missing in v2; private v1 account paths exist | Authenticated v2 balances stream | P2 |
| `executions` | Missing in v2; v1 `ownTrades`/`openOrders` exist | Authenticated v2 order/trade event stream | P2 |

## Implementation phases

### Phase 0 — Protocol inventory and tests first

Create fixture-driven parser tests using Kraken doc examples and saved live frames.

Actions:

1. Add `typhoon-engine/src/broker/kraken/ws_v2.rs`.
2. Move shared v2 endpoint/request-id/ping/backoff pieces out of `ohlc_ws.rs` only if this does not churn the existing streamer.
3. Add test fixtures under `typhoon-engine/src/broker/kraken/fixtures/ws_v2/`:
   - `ticker_snapshot.json`
   - `ticker_update.json`
   - `book_snapshot.json`
   - `book_update.json`
   - `trade_snapshot.json`
   - `level3_snapshot.json`
   - `level3_update.json`
   - `balances_snapshot.json`
   - `executions_snapshot.json`
4. Add parser tests before wiring streams.

Verification:

- `cargo test -p typhoon-engine broker::kraken::ws_v2`
- `cargo check -p typhoon-engine`

### Phase 1 — L1 ticker

Status: engine parser + stream driver foundation implemented (2026-07). Native subscription reconciliation and UI wiring **completed** in subsequent updates (see L3 foundation section and ADR-129).

Implement `ws_v2_ticker.rs` and native active-symbol streamer.

Data model:

```rust
pub struct KrakenWsTicker {
    pub symbol: String,
    pub bid: Option<f64>,
    pub bid_qty: Option<f64>,
    pub ask: Option<f64>,
    pub ask_qty: Option<f64>,
    pub last: Option<f64>,
    pub volume_24h: Option<f64>,
    pub vwap_24h: Option<f64>,
    pub low_24h: Option<f64>,
    pub high_24h: Option<f64>,
    pub change_24h: Option<f64>,
    pub change_pct_24h: Option<f64>,
    pub ts_ms: Option<i64>,
    pub is_snapshot: bool,
}
```

Native behavior:

- Subscribe only to active/focused symbols:
  - current chart symbol
  - watchlist symbols visible in UI
  - open MTF Grid symbols
  - held positions
- Coalesce updates to ~10-20 Hz max before sending UI messages.
- Use ticker as quote/last-price feed, not as historical bar storage.

Verification:

- Parser tests for string/number numeric fields.
- Live smoke behind config flag: subscribe to `BTC/USD`, wait for snapshot/update, assert bid/ask or last exists.

### Phase 2 — L2 order book v2 replacement

Status: engine parser/state helper + stream driver foundation implemented (2026-07). CRC32 checksum validation, resync policy, and native replacement wiring **completed** in subsequent updates (v2 primary; v1 kept for compat — see Update sections and ADR-129).

Implement `ws_v2_book.rs` and replace `public_book.rs` v1 path.

Requirements:

- Subscribe to `book` on `wss://ws.kraken.com/v2`.
- Support configurable depth.
- Parse snapshot and update frames.
- Maintain sorted bid/ask state.
- Remove levels when qty is zero.
- Validate Kraken CRC32 checksum for top 10 bids/asks.
- On checksum mismatch:
  - mark book degraded
  - unsubscribe/resubscribe or reconnect
  - surface a status message
- Keep v1 `public_book.rs` temporarily until v2 parity is verified, then retire it.

Native behavior:

- L2 is active/focused symbol only, not full universe.
- UI should show source `kraken_ws_v2_book`, depth, checksum status, last update age.

Verification:

- Existing v1 book tests ported to v2 fixture shape.
- Checksum-good fixture passes.
- Checksum-bad fixture fails and requests resync.
- Manual live check: `BTC/USD` depth 10 book updates without crossed book.

### Phase 3 — Trades stream

Implement `ws_v2_trade.rs`.

Use cases:

- time-and-sales panel
- last-trade event feed
- validating OHLC/ticker freshness
- potential future tick-derived indicators

Behavior:

- Active/focused symbols only.
- Keep in-memory ring buffer per symbol; do not persist every trade by default.
- Optional bounded persistence table can be added later if needed.

Verification:

- Parser tests for snapshot last 50 trades and live update frames.
- Live smoke for one symbol.

### Phase 4 — v2 authenticated account streams

Implement `ws_v2_account.rs` for:

- `balances` on `wss://ws-auth.kraken.com/v2`
- `executions` on `wss://ws-auth.kraken.com/v2`

Why:

- v2 `executions` supersedes the current v1 split between `ownTrades` and `openOrders`.
- v2 `balances` provides live account balance/ledger changes, which current private v1 support does not fully cover.

Behavior:

- Use existing token retrieval flow if compatible; otherwise add v2 token acquisition wrapper using current secure credential handling.
- Never log tokens.
- Redact all auth errors that might contain credentials.
- Keep v1 private WS as fallback until v2 has soak time.

Verification:

- Fixture tests for snapshots and updates.
- Local auth smoke: connect, subscribe, receive ACK/snapshot, then disconnect.
- Existing account/position/order UI stays consistent.

### Phase 5 — Level 3 opt-in order book

`ws_v2_level3.rs` + native integration foundation complete (2026-07 update below).

Important constraints from Kraken docs (still apply):
- Endpoint is `wss://ws-l3.kraken.com/v2`.
- Channel is authenticated.
- There are symbol-count and subscription-rate limits.
- Subscription cost depends on depth.
- Only one depth subscription per symbol is supported.

Implemented behavior (foundation):
- Explicit opt-in per symbol via `KrakenStartLevel3Ws` (demo/sim always available).
- Maintain individual visible orders (per-order_id state + deltas).
- CRC checksum validation + status (connected / subscribed / real-feed CRC OK/MISMATCH / demo).
- L3 deltas project to same update paths as L2 (KrakenOrderbookUpdate + BookQuoteTick) for cross-checking / consumers.
- Bookmap + depth profile integration with per-order markers, list, age coloring.

Remaining (P2/P3, requires entitlements):
- Enforce subscription budget in client UI before frames.
- Richer dedicated L3 status panel (beyond current KrakenWsStatus events).
- Live-only smoke / more projections when entitled.
- Do not auto-enable at startup (already respected).

Verification (foundation):
- Unit test + sim exercises parse/apply/CRC/order-id behavior.
- Checksum validation exercised in test and sim.
- Sim path for UI/dev; real requires token + entitlements (documented).
- Live smoke for one symbol/depth only.

## UI/product impact

Add a Kraken market-data status panel section:

- L1 ticker: connected/subscribed count/update age
- L2 book: symbol/depth/checksum/update age
- Trades: ring buffer count/update age
- L3: disabled/enabled/authenticated/depth/budget/checksum
- Account v2: balances/executions connected + fallback status

This should not pollute chart rendering. Streams should update state caches and publish coalesced `BrokerMsg` events.

## Performance rules

- OHLC full-universe streaming remains separate and coalesced.
- L1/L2/trade are active-symbol only.
- L3 is opt-in only.
- No per-tick SQLite writes for ticker/trade/book updates.
- UI dispatch must batch/coalesce updates and avoid per-frame allocations where possible.
- Parser modules should be small, tested, and independent to reduce compile churn.

## Suggested next coding step

Start with Phase 1 + Phase 2 parser layer only:

1. Add `ws_v2.rs`, `ws_v2_ticker.rs`, and `ws_v2_book.rs`.
2. Add subscribe-frame builders and parser tests.
3. Do not wire native UI yet.
4. Run:
   - `cargo test -p typhoon-engine broker::kraken::ws_v2`
   - `cargo test -p typhoon-engine broker::kraken::ws_v2_ticker broker::kraken::ws_v2_book`
   - `cargo check -p typhoon-engine`
5. Then wire ticker/book stream supervisors in native.

## Acceptance criteria

- Kraken WS v2 L1 ticker is live for active symbols.
- Kraken WS v2 L2 book replaces v1 public book and validates checksums.
- Kraken WS v2 trades stream is available for active symbols.
- Kraken WS v2 account `balances` and `executions` are implemented with v1 fallback.
- Kraken WS v2 L3 is available as explicit opt-in with budget/status visibility.
- Existing OHLC full-universe streaming continues to work and compile cleanly.
- All new parser/state modules have fixture-backed unit tests.
- `cargo check --workspace` passes.

## Update 2026-07: L2 Book Robustness Complete (atomic checksum, O(1) dispatch, Alpaca symmetry)

- `ws_v2_book.rs`: Full `KrakenWsBookState` with `apply_delta_with_checksum` using **candidate state** (clone + apply + CRC match before commit). Only commits on exact match. `compute_checksum` uses exact wire `price_text`/`qty_text` tokens (critical for xStock/METAx trailing-zero CRCs; live pinned test).
- Bounded resub (max 10 consecutive mismatches), exp backoff, ping/pong heartbeat, batched subscribe with delay, unsubscribe frames, event reporting (Connected/Subscribed/Disconnected/Mismatch).
- Native: `KrakenStartOrderbookWs` + runtime streamer uses v2 `run_book_streamer`; top-of-book emits `KrakenBookQuoteTick`; integrated into O(1) `chart_by_bare` / `watchlist_by_bare` dispatch + `handle_kraken_book_quote_tick`.
- v1 `public_book.rs` kept for legacy/compat (mod.rs still wires both); v2 is primary for new L2 features (orderbook window, chart L2).
- Parity with Alpaca WS hardening (feed-aware caps, 406/limit backoff, ack surfacing, diff sub/unsub, reconnect hygiene) — both brokers now have strong WS robustness + O(1) native quote paths.
- Tests: checksum match/mismatch, snapshot+delta, live xStock fixture, backoff bounds.

L2 book phase (Phase 2) is complete for core + robustness. Remaining phases (trade, L3, account v2 migration) remain P2 per original.

## Update 2026-07: L3 Foundation Complete (parser/streamer/CRC/state/viz)

- `ws_v2_level3.rs`: Full implementation of `KrakenL3Level` (incl. `received_at_ms` for runtime age persistence even without wire timestamp), `KrakenL3Delta`, `parse_l3_message` (supports snapshot + deltas with order_id / limit_price / order_qty / timestamp), `KrakenL3State` (per-order add/mod/delete by order_id), `compute_l3_checksum` + `KrakenL3ChecksumError` + `apply_delta_with_checksum` (candidate clone + exact match commit only, mirroring L2 book exactly; top-10 levels, text preservation).
- Real-feed CRC on live deltas: always routes checksum-present messages through validation in streamer (real auth path + sim fallback for demo/no-token). Mismatch status + resilience (forward delta; prod can resub).
- Auth + wiring: `run_level3_streamer` / `once` accept `Option<String> token`; subscribe includes token when present; modeled on private_ws + ws_v2_book. `KrakenStartLevel3Ws` in runtime fetches token via `get_websockets_token` and spawns.
- Real consume: WS text frames parsed and emitted as `KrakenL3Delta` (converted to same JSON paths as L2: `KrakenOrderbookUpdate`, `KrakenBookQuoteTick`).
- Sim fallback: `simulate_l3_delta` exercises the full path (with checksums for CRC tests).
- State exposure: `KrakenL3State` exported; maintained inside streamer loop and runtime command handler; status events carry CRC OK / MISMATCH / subscribe info.
- Bookmap + DOM: is_l3 detection, per-order markers, richer scroll list pane (order_id truncated, price×qty, side color, copy on click/row, age "new/mid/old" labels + interactions), age-based coloring (newer = brighter bars; backed by wire timestamp + `received_at_ms`).
- Depth + charts: 25 levels from L3 propagated to `live_depth_bids/asks`; "L3 depth" label with distinct tint in overlay; MTF Grid parity via existing `chart_by_bare` O(1) dispatch (depth updates hit MTF charts for the symbol).
- Limits documented everywhere: real L3 requires Kraken entitlements + auth token; sim always works for UI/dev.
- Test: `l3_state_apply_and_checksum_basic` (snapshot → qty modify → delete; asserts state + CRC).
- Status surface: events like "L3 real-feed CRC OK ...", "L3 real-feed CRC MISMATCH ...", "L3 connected (auth path)" / demo.

L3 foundation (Phase 5 core) is complete and integrated with existing L1/L2 paths for zero-delta consumption. Full native dedicated L3 status panel, per-symbol budget UI enforcement, and live-only verification remain P2/P3 (require entitlements for smoke). Sim/demo path keeps everything exercisable.

Regression notes from prior work preserved. Re-open for trade stream, account v2, or full L3 panel when needed.
