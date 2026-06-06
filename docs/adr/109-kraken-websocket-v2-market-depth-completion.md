# ADR-109: Kraken WebSocket v2 Market Depth Completion

Status: Proposed
Date: 2026-06-06

## Context

TyphooN currently uses Kraken WebSocket v2, but not comprehensively.

Source audit:

- `engine/src/broker/kraken/ohlc_ws.rs`
  - uses `wss://ws.kraken.com/v2`
  - supports public `ohlc`
  - builds v2 subscribe/unsubscribe frames
  - parses snapshot/update OHLC frames
  - runs one reconnecting streamer per interval
- `engine/src/broker/kraken/mod.rs`
  - uses v2 `instrument` snapshot for Kraken tokenized equity/xStocks universe discovery
- `engine/src/broker/kraken/public_book.rs`
  - still uses public WebSocket v1 endpoint `wss://ws.kraken.com`
  - subscribes to v1 `book` / `book-N`
  - parses v1 array frames with `as`/`bs` snapshot keys and `a`/`b` delta keys
- `engine/src/broker/kraken/private_ws.rs`
  - private account feed is v1 shape: `ownTrades` and `openOrders`
  - no v2 `executions` or `balances` channel parser yet
- `native/src/app/kraken_ohlc_ws.rs`
  - app-level full-universe OHLC streamer/write path only; no L1/L2/L3 market-data dispatcher yet

Kraken WebSocket v2 documentation exposes more than we currently consume:

- Level 1 public market data: `ticker` on `wss://ws.kraken.com/v2`
- Level 2 public market data: `book` on `wss://ws.kraken.com/v2`
- Level 3 authenticated market data: `level3` on `wss://ws-l3.kraken.com/v2`
- Trades: `trade` on `wss://ws.kraken.com/v2`
- Candles: `ohlc` on `wss://ws.kraken.com/v2` — already implemented
- Instruments: `instrument` on `wss://ws.kraken.com/v2` — snapshot path already implemented
- Authenticated account streams: `balances` and `executions` on `wss://ws-auth.kraken.com/v2`

Conclusion: we do **not** support 100% of Kraken WebSocket v2. We support OHLC and one-shot instrument snapshot; order book support exists but is v1 L2; private user-stream support exists but is v1 private WS.

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

Under `engine/src/broker/kraken/`:

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

- `native/src/app/kraken_market_ws.rs`
  - app-level supervisor for active/focused symbol L1/L2/trade streams
  - symbol subscription reconciliation when chart/watchlist/MTF focus changes
  - coalesced UI events so egui is not spammed per tick
- `native/src/app/kraken_l3_ws.rs`
  - opt-in L3 supervisor
  - only for selected symbols/depth
  - clear status/errors/rate-budget display

## Coverage matrix

| Kraken WS v2 channel | Current state | Target state | Priority |
| --- | --- | --- | --- |
| `ohlc` | Implemented and full-universe streamed | Keep; share future common v2 helpers only if low-risk | Done / preserve |
| `instrument` | One-shot snapshot implemented | Keep; optionally move parsing to channel module | Done / cleanup |
| `ticker` L1 | Missing | Active/focused symbols; drives quote cards/watchlist/last-price/top-of-book | P1 |
| `book` L2 | v1 implementation exists | v2 implementation with checksum; replace v1 public book path | P1 |
| `trade` | Missing | Active/focused symbols; time-and-sales, tick feed, last-trade updates | P2 |
| `level3` L3 | Missing | Authenticated, opt-in, depth/symbol-budgeted visible order book | P2/P3 |
| `balances` | Missing in v2; private v1 account paths exist | Authenticated v2 balances stream | P2 |
| `executions` | Missing in v2; v1 `ownTrades`/`openOrders` exist | Authenticated v2 order/trade event stream | P2 |

## Implementation phases

### Phase 0 — Protocol inventory and tests first

Create fixture-driven parser tests using Kraken doc examples and saved live frames.

Actions:

1. Add `engine/src/broker/kraken/ws_v2.rs`.
2. Move shared v2 endpoint/request-id/ping/backoff pieces out of `ohlc_ws.rs` only if this does not churn the existing streamer.
3. Add test fixtures under `engine/src/broker/kraken/fixtures/ws_v2/`:
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

Implement `ws_v2_level3.rs` and native L3 panel/diagnostics.

Important constraints from Kraken docs:

- Endpoint is `wss://ws-l3.kraken.com/v2`.
- Channel is authenticated.
- There are symbol-count and subscription-rate limits.
- Subscription cost depends on depth.
- Only one depth subscription per symbol is supported.

Behavior:

- Explicit opt-in per symbol/depth.
- Do not auto-enable at startup.
- Maintain individual visible orders, not aggregated price levels.
- Enforce subscription budget in client before sending frames.
- Show clear status:
  - connected
  - subscribed
  - throttled by local budget
  - rejected by Kraken
  - checksum mismatch/resyncing
- Provide an aggregated L2 projection from L3 state for cross-checking.

Verification:

- Fixture tests for snapshot/update/delete/order-id behavior.
- Checksum validation.
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
