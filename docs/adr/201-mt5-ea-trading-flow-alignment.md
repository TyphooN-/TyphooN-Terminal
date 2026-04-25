# ADR-201: MT5 EA Trading-Flow Alignment Across Brokers

**Date:** 2026-04-24
**Status:** Accepted
**Related:** `engine/src/broker/alpaca.rs`, `engine/src/broker/tastytrade.rs`, `engine/src/broker/kraken_broker.rs`, `native/src/app.rs`, ADR-010 (multi-broker), ADR-022 (tastytrade), ADR-072 (kraken)

## Context

The terminal already routes orders to three brokers — Alpaca (equity / crypto),
tastytrade (equity / options), and Kraken (crypto) — through the
`BrokerCmd` / `BrokerMsg` channel architecture from ADR-010. Each broker has
its own native model for positions and orders:

- **Alpaca**: per-fill positions exposed as a flat list, bracket orders carry
  a `legs` array of child stop/limit orders, REST `close_position` rejects with
  `insufficient qty available` when an open exit order still holds the size.
- **tastytrade**: per-leg positions surfaced through a multi-leg order DSL,
  open exit orders are not cancelled when a flat-out close is submitted.
- **Kraken**: positions surface as raw `OpenPositions` rows keyed by `pair`,
  with separate `vol`, `vol_closed`, `cost`, `value`, `net` fields and no
  notion of a "net position per symbol".

The TyphooN MT5 EA, which is the reference trading surface for the desk,
expects a far simpler and stricter model:

1. **One net position per symbol.** Multiple buy/sell fills on the same symbol
   collapse to one signed quantity.
2. **Partial close at a specified volume.** `PositionClosePartial(ticket, vol)`
   reduces the net position by `vol` units without touching anything else.
3. **Close all positions.** `PositionsCloseAll()` flattens the book in one
   call.
4. **`PositionClose()` cancels SL/TP first.** Pending exit orders are tied to
   the position and are auto-cancelled when the position closes; the EA never
   sees a `not enough qty` reject because the exit order was holding it.
5. **Display symbols match the EA's symbol table** — `BTCUSD`, not Kraken's
   internal `XBTUSD` / `XXBTZUSD`.

Without an alignment pass, every cross-broker workflow (close from chart,
close from sync window, EA-style automation) had to special-case each broker's
quirks.

## Decision

Align all three brokers to the MT5 EA's position-and-exit semantics.

### 1. Net-position summary, not raw fills

Add a per-broker accessor that returns positions in the shared
`alpaca::PositionInfo` shape: `{ symbol, qty, side, avg_entry_price,
market_value, unrealized_pl, asset_class, asset_id }`.

- `KrakenBroker::get_position_summaries()` — sums `vol - vol_closed` per pair
  across all legs (signed by `type`), maps `XBTUSD` → `BTCUSD`,
  `XDGUSD` → `DOGEUSD` via `display_pair()`, derives `avg_entry` from
  `cost / volume`.
- `TastytradeBroker::position_summaries()` already existed in this shape;
  call sites confirmed.
- Alpaca's native shape becomes the canonical contract.

This is what the chart pane, sync status window, and broker handlers all read
from now — no broker-native shapes leak out.

### 2. Partial close

Each broker exposes a `close_*_position_qty(symbol, qty)` that reduces the
net position by `qty`, capped at the current open volume. Full close becomes
a thin wrapper that passes `qty = current_open_volume`.

- **Alpaca**: existing `close_position(symbol, qty: Option<f64>, ...)` already
  supported partial qty; the wrapper now uses the cancel-then-close flow
  below.
- **tastytrade**: new `close_equity_position_qty(symbol, qty)` — looks up the
  position, derives `Sell to Close` / `Buy to Close` action from
  `quantity_direction`, places a market order at `close_qty`.
- **Kraken**: `close_position(pair, ..., qty)` already accepted size; the
  net-volume calculation moved to `Self::net_position_volume()` so partial
  and full close share one path.

### 3. Close all

- **tastytrade**: `close_all_equity_positions()` iterates `get_positions()`
  and submits one close per non-zero leg, returning the count closed.
- **Alpaca**: existing `close_all_positions` retained; surfaced through the
  same UI affordance.
- **Kraken**: equivalent path via repeated `close_position` per pair from the
  position-summary list.

### 4. Cancel-pending-exit-orders-before-close

This is the most behaviorally important change. On every close path:

1. List open orders for the symbol that are still cancellable
   (status not in `filled / canceled / expired / rejected`).
2. Walk bracket-order `legs` to find child stop / limit orders attached to
   the parent.
3. Cancel them before submitting the close order.
4. If the close still rejects with `insufficient qty available`, retry once
   after re-listing cancellable exits — handles the race where a fill
   completed between listing and submission.

Implemented as:

- Alpaca: `cancel_open_orders_for_symbol()` +
  `collect_cancellable_order_ids_for_symbol()` (recursive walk over `legs`)
  + `close_position_once()` (single attempt) wrapped by the retrying
  `close_position()`.
- tastytrade: `cancel_live_exit_orders_for_symbol()` invoked from
  `close_equity_position_qty()` before placing the close order.
- Kraken: `cancel_live_exit_orders_for_pair()` invoked from `close_position`
  before reducing the position.

`order_status_is_cancellable` and `is_insufficient_qty_close_reject` are
shared classification helpers so each broker uses the same definition of
"still attached to the position".

### 5. Display-symbol normalization

`KrakenBroker::display_pair()` translates internal pair codes to EA-visible
symbols. The chart, position table, and sync windows all consume display
symbols — so a Kraken `XBTUSD` and an MT5 `BTCUSD` reconcile to the same row.

## Consequences

- **One UX path closes positions on any broker.** The chart's right-click
  Close, the sync window's per-row close button, and any future automation
  surface go through one cancel-exits-then-close flow.
- **No more `insufficient qty` user-facing failures.** Pending SL / TP no
  longer block close. The retry-once policy covers the race window where a
  cancel acknowledgement and a fill cross.
- **Position display matches the MT5 EA's symbol table.** Kraken positions
  show as `BTCUSD` / `DOGEUSD`, not `XBTUSD` / `XDGUSD`.
- **Net-position semantics imply data loss on the close path.** For brokers
  that internally hold separate fills (Kraken), the partial-close size targets
  the net, not a specific lot. This is the EA's model and is intentional.
- **Future brokers must implement the same surface.** Any new broker added
  under ADR-010 has to expose `get_position_summaries`,
  `close_*_position_qty`, `close_all_*_positions`, and a cancel-exits hook
  before it can hook into the chart's close affordance.

## Validation

- `cargo build --workspace` clean across all crates.
- `cargo test --workspace --lib` — 1905 tests pass (216 mql5-compiler,
  1632 engine, 57 web-protocol).
- Manual: close-from-chart on a tastytrade equity position with a live SL
  attached — exit order cancels, position flattens in one click.
- Manual: close-from-chart on Kraken `BTCUSD` net of two opposing fills —
  net qty calculated correctly, partial close submits the correct side.
