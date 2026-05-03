# ADR-072: Kraken as Full Broker (Data + Trading)

**Status:** Accepted | **Date:** 2026-05-01

## Context

Kraken is the terminal's crypto exchange integration. TyphooN uses it in two
separate ways:

- `engine/src/core/kraken.rs` fetches public Spot/xStocks OHLCV bars.
- `engine/src/core/kraken_futures.rs` fetches public Futures instruments and
  chart candles.
- `engine/src/broker/kraken_broker.rs` owns authenticated account and order
  REST calls.

The official Spot REST surface includes market data, account data, trading,
funding, subaccounts, earn, and transparency endpoints. The trading surface
includes `AddOrder`, `AmendOrder`, `CancelOrder`, `CancelAll`,
`CancelAllOrdersAfter`, `GetWebSocketsToken`, `AddOrderBatch`,
`CancelOrderBatch`, and `EditOrder`.

## Decision

Kraken remains a first-class broker beside Alpaca and tastytrade. The engine
centralizes Kraken nonce generation, request signing, form encoding, response
error handling, pair normalization, and order construction in
`KrakenBroker`.

Signed requests follow Kraken's REST authentication scheme:

- `API-Key` header carries the public key.
- `API-Sign` is `HMAC-SHA512(uri_path + SHA256(nonce + POST data))` using the
  base64-decoded API secret.
- `nonce` is generated monotonically per broker instance.
- Form bodies are percent-encoded before signing so fields such as
  `close[ordertype]`, `+2%`, client order IDs, and batch payloads sign exactly
  as submitted.

## Order Coverage

`KrakenOrderRequest` models the full AddOrder mechanism used by Spot REST and
authenticated WebSocket v1:

- Order types: `market`, `limit`, `iceberg`, `stop-loss`,
  `stop-loss-limit`, `take-profit`, `take-profit-limit`, `trailing-stop`,
  `trailing-stop-limit`, `settle-position`.
- Primary and secondary prices: `price`, `price2`, including relative strings
  such as `+2%`.
- Iceberg display size: REST `displayvol`; `iceberg` is accepted by the typed
  request and submitted as `ordertype=limit` with `displayvol`, matching
  Kraken REST examples.
- Margin settlement: `settle-position` accepts `volume=0` so margin positions
  can be settled without precomputing exact remaining size.
- Margin controls: `leverage`, `margin`, `reduce_only`.
- Flags: `oflags` (`post`, `fciq`, `fcib`, `nompp`, `viqc`).
- Scheduling and expiry: `starttm`, `expiretm`, `deadline`, `timeinforce`.
- Client identifiers: `cl_ord_id`, `userref`, `sender_sub_id`, `reqid`.
- Self-trade prevention: `stp_type`.
- Dry-run validation: `validate=true`.
- Conditional OTO close fields: `close[ordertype]`, `close[price]`,
  `close[price2]`.

The older helper `place_order_with_leverage()` remains for simple callers but
delegates to `KrakenOrderRequest`, so all new validation and encoding behavior
is shared.

## REST Endpoint Coverage

Typed or pass-through wrappers exist for the actively used account/trading
surface:

- Account: `Balance`, `BalanceEx`, `TradeBalance`, `OpenOrders`,
  `ClosedOrders`, `QueryOrders`, `OrderAmends`, `TradesHistory`,
  `QueryTrades`, `OpenPositions`, `Ledgers`, `QueryLedgers`, `TradeVolume`,
  `GetApiKeyInfo`.
- Trading: `AddOrder`, `AddOrderBatch`, `AmendOrder`, `EditOrder`,
  `CancelOrder`, `CancelOrderBatch`, `CancelAll`, `CancelAllOrdersAfter`,
  `GetWebSocketsToken`.
- Public: `AssetPairs` for Spot/xStocks pair discovery; Spot/xStocks OHLC
  remains in `core/kraken.rs`; Kraken Futures instruments and chart candles
  remain in `core/kraken_futures.rs`.

For less common Kraken REST endpoints, `private_post_owned()` is intentionally
public inside the broker module API. This keeps signing and nonce handling
centralized while allowing funding, earn, subaccount, and export/report calls
to be added without copying authentication code.

## Public Bar Sync

Kraken Spot/xStocks and Kraken Futures public market-data fetches are
asynchronous and bounded by a shared public semaphore. As of ADR-210, the
terminal keeps 16 public Kraken requests in flight, uses larger Spot/Futures
queue windows, offloads cache merge/write work to blocking tasks, and runs the
Kraken leg of combined CryptoCompare backfills concurrently with CryptoCompare
pagination. This keeps recent Kraken bars arriving as soon as possible while
CryptoCompare continues to fill deeper history.

## UI And Web Routing

Native quick-trade and chart-position controls can route crypto orders to
Kraken. Close-all, partial-close, cancel-order, and exit synchronization use
the same MT5 EA semantics as Alpaca and tastytrade (ADR-201).

The LAN web/mobile protocol now accepts `kraken` for order, cancel, and close
commands. Web order types are normalized to Kraken names:

- `stop` / `stop_loss` -> `stop-loss`
- `stop_limit` / `stop_loss_limit` -> `stop-loss-limit`
- `take_profit` -> `take-profit`
- `take_profit_limit` -> `take-profit-limit`
- `trailing_stop` -> `trailing-stop`
- `trailing_stop_limit` -> `trailing-stop-limit`

If the web/mobile order includes stop-loss or take-profit bracket fields,
TyphooN submits the entry order and then queues a Kraken exit sync once the
position is visible.

## Consequences

- **Pro:** Kraken order support now covers the documented order-type matrix,
  margin/reduce-only controls, time-in-force, post-only/fee flags, validation
  mode, client IDs, STP, conditional close fields, batch add/cancel, amend, and
  edit.
- **Pro:** Signed form encoding is tested against Kraken's published
  `AddOrder` signature vector.
- **Pro:** Mobile/web order routing no longer treats Kraken as close/cancel
  only.
- **Con:** Kraken's REST API does not provide a native two-leg OCO bracket for
  both SL and TP in one `AddOrder`; TyphooN continues to place and resync exit
  orders independently for MT5-style SL+TP behavior.
- **Con:** Funding, earn, subaccount, and export/report workflows are broker
  API-capable through `private_post_owned()` but still need dedicated UI
  surfaces before users can operate them directly from the terminal.

## References

- Kraken Spot REST authentication guide:
  https://docs.kraken.com/api/docs/guides/spot-rest-auth/
- Kraken Spot REST Add Order:
  https://docs.kraken.com/api/docs/rest-api/add-order/
- Kraken Spot REST Add Order Batch:
  https://docs.kraken.com/api/docs/rest-api/add-order-batch/
- Kraken WebSocket v1 Add Order parameter matrix:
  https://docs.kraken.com/api/docs/websocket-v1/addorder/
