# ADR-015: Full Order Management

**Status:** Implemented
**Date:** 2026-03-15

## Context

The terminal only supported market orders. MT5 and Godel Terminal support limit, stop, stop-limit, trailing stop, and bracket orders. Alpaca's REST API supports all of these via `POST /v2/orders`.

## Decision

Implement all 6 Alpaca order types, plus order history, modify, and cancel. Use a single order type selector dropdown in the UI.

## Order Types

| Type | Alpaca API | TyphooN Frontend |
|---|---|---|
| Market | `"type": "market"` | Default — instant fill |
| Bracket | `"order_class": "bracket"` + TP/SL legs | Auto when SL+TP set — broker-enforced stops |
| Limit | `"type": "limit"` + `limit_price` | Uses current price as limit |
| Stop | `"type": "stop"` + `stop_price` | Uses SL line price |
| Stop-Limit | `"type": "stop_limit"` + both prices | Uses SL as stop, TP as limit |
| Trailing Stop | `"type": "trailing_stop"` + `trail_price` | Trail distance = |price - SL| |

## Backend (Rust)

New `AlpacaBroker` methods:
- `limit_order()`, `stop_order()`, `stop_limit_order()`, `trailing_stop_order()`, `bracket_order()` — all use `submit_order()` common helper
- `get_orders(status, limit)` — `GET /v2/orders` for open/closed
- `modify_order(id, qty, limit, stop, trail)` — `PATCH /v2/orders/{id}`
- `cancel_order(id)` — `DELETE /v2/orders/{id}`

New `OrderInfo` struct with full order details (type, prices, fill info, timestamps).

10 new Tauri commands registered, all with input validation.

## Frontend

- **Order type selector** in `#order-config`: dropdown with 6 options
- **Open Trade button** dispatches to correct backend command based on selector
- **Positions panel**: live P/L per position, one-click close, click to switch chart
- **Orders panel**: open orders with cancel buttons, recent fills with prices
- **Smart Close Partial**: floating window with qty input + 25/50/75/100% quick buttons
- **Button debounce**: `orderInFlight`, `closeAllInFlight`, `closePartialInFlight`, `mgInFlight` guards

## Consequences

- **Pro**: Full order type coverage — matches MT5 functionality
- **Pro**: Bracket orders = broker-enforced SL/TP (no local tracking needed)
- **Pro**: Smart close partial replaces crude `prompt()` with visual UI
- **Pro**: Positions/orders panels give full account visibility without leaving the chart
- **Con**: More UI complexity in right panel (mitigated by collapsible panels)
