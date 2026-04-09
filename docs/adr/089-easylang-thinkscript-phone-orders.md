# ADR-089 — EasyLanguage + thinkScript Compilers, Phone Order Entry

**Status:** Accepted
**Date:** 2026-04-09

## Context

Three items were explicitly deferred from ADR-069 and ADR-073 as "large scope, not critical path":

1. **EasyLanguage compiler** (third frontend for MQL5 IR pipeline)
2. **thinkScript compiler** (fourth frontend)
3. **ADR-073 Phase 2 mobile order entry** (place / cancel / close from phone)

This ADR lands all three. The rationale for finally tackling them is simple: the IR + codegen infrastructure already exists for MQL5 and PineScript, and adding a new frontend is mostly a line-scanner + expression parser that emits `IrStmt`/`IrExpr` nodes. The marginal cost of each new language is low once the shared pipeline exists.

## Decisions

### 1. EasyLanguage frontend (`mql5-compiler/src/easylang.rs`)

Line-based scanner modeled after `pine.rs`. Covers the ~90% of community EL indicators:

**Supported:**
- `inputs:` block (multi-line, comma-separated, `Name(default)` form)
- `variables:` / `vars:` block
- Case-insensitive keywords and identifiers (EL convention)
- Built-in series: `Close`, `Open`, `High`, `Low`, `Volume`, `C`, `H`, `L`, `O`, `V` shortcuts, `CurrentBar`/`BarNumber`
- Built-in functions mapped to common IR calls:
  - `Average` / `Avg` / `SMA` → `ta_sma`
  - `XAverage` / `EMA` → `ta_ema`
  - `RSI` → `ta_rsi`
  - `ATR` → `ta_atr`
  - `Highest` / `Lowest` → `ta_highest` / `ta_lowest`
  - `StdDev` / `StandardDev` → `ta_stdev`
  - `AbsValue` / `Absolute` / `Abs` → `math_abs`
  - `SquareRoot` / `Sqrt` → `math_sqrt`, `Log` → `math_log`
  - `Max` / `Min` → `math_max` / `math_min`
- Assignment statements: `Name = expression;`
- `Plot1..PlotN(value, "label")` — label extracted for metadata
- Arithmetic, comparison, `=` (eq), `<>` (ne) operators
- Parenthesised sub-expressions
- `{ ... }` multi-line brace comments
- `// ...` line comments

**Deferred:**
- `if/then/else` multi-line blocks (single-line statements work via assignment)
- `Buy`/`Sell`/`SellShort`/`BuyToCover` trade signals (no trade sim path yet)
- User-defined functions
- Arrays

**11 tests** cover: simple plot, multi-input, multi-plot, brace comments, line comments, case-insensitivity, binary ops, empty source, series shortcuts, `XAverage` → `ta_ema` mapping, nested-paren comma splitting.

### 2. thinkScript frontend (`mql5-compiler/src/thinkscript.rs`)

Parallel structure to EL, but case-sensitive per thinkScript convention.

**Supported:**
- `input name = default;` (int/float/bool inferred — `yes`/`no` → bool, integer literal → I32, float literal → F64, series reference → F64 with 0.0 default)
- `def name = expression;`
- `plot name = expression;` — the `name` becomes the plot label
- `declare lower;` / `declare upper;` — toggle the `separate_window` metadata flag
- `# ...` line comments (thinkScript uses hash)
- Built-in series (case-sensitive: `close`, `open`, `high`, `low`, `volume`)
- Built-in functions (case-sensitive, documented spellings):
  - `Average` / `MovingAverage` / `SimpleMovingAvg` → `ta_sma`
  - `ExpAverage` / `ExponentialMovingAvg` → `ta_ema`
  - `RSI` → `ta_rsi`
  - `ATR` / `TrueRange` → `ta_atr`
  - `Highest` / `Lowest` → `ta_highest` / `ta_lowest`
  - `StDev` / `StandardDev` → `ta_stdev`
  - `AbsValue` / `Abs` → `math_abs`
  - `Sqrt` / `SquareRoot` → `math_sqrt`, `Log` → `math_log`
  - `Max` / `Min` → `math_max` / `math_min`
- Assignment re-binding: `name = expr;` (outside `def`, same semantics)
- Arithmetic + comparison operators

**Deferred:**
- `plot.Color` / `AssignValueColor` (colors default to blue)
- Multi-line `if then else`
- Arrays / reference arrays
- `script` function definitions

**12 tests** cover: simple MA, multi-input, multi-plot, `declare lower`, comment stripping, `ExpAverage` → `ta_ema` mapping, bool input (`yes`/`no`), float input, arithmetic expressions, def-then-plot sequencing, empty source, comparison-in-assignment (`x = close == high` shouldn't trip on the `==`).

### 3. UI integration (Indicator Compiler window)

- **Language dropdown** expanded from 2 to 4 entries (MQL5 / PineScript v5 / EasyLanguage / thinkScript).
- **File loader** accepts `.el`, `.els` (EasyLanguage), `.ts`, `.tos` (thinkScript) in addition to `.mq5`/`.mqh`/`.pine`/`.txt`.
- Auto-detection on load: `.pine` → PineScript, `.el`/`.els` → EasyLanguage, `.ts`/`.tos` → thinkScript, everything else → MQL5.
- `COMPILE` command description updated to list all four languages.

### 4. Phone order entry (ADR-073 Phase 2)

Three new variants on `WebCmd`:

```rust
PlaceOrder {
    symbol: String,
    qty: f64,
    side: String,         // "buy" | "sell"
    order_type: String,   // "market" | "limit" | "stop"
    limit_price: Option<f64>,
    stop_price: Option<f64>,
    broker: String,       // "alpaca" | "tastytrade"
}
CancelOrder { order_id: String, broker: String }
ClosePosition { symbol: String, broker: String }
```

Plus a corresponding reply variant on `WebMsg`:

```rust
OrderResult { ok: bool, message: String }
```

**Validation (web-server `run_websocket_session`):**
Each new command is pattern-matched in the dispatch loop and validated before being relayed to the native app:
- `PlaceOrder`: symbol format, qty bounds (0 < q ≤ 100,000, finite), side whitelist, order_type whitelist, broker whitelist (`alpaca`/`tastytrade`), limit/stop prices finite and positive
- `CancelOrder`: order_id length (≤64), alphanumeric + `-` + `_` only, broker whitelist
- `ClosePosition`: symbol format, broker whitelist

Invalid commands are dropped with a `tracing::warn!` and never reach the native app.

**Native relay (`native/src/app.rs` web cmd drain):**
- `PlaceOrder` → translates to the appropriate `BrokerCmd::AlpacaMarketOrder` / `AlpacaLimitOrder` / `AlpacaStopOrder` / `TastytradeEquityOrder` depending on broker + type
- `CancelOrder` → `BrokerCmd::AlpacaCancelOrder` (tastytrade cancel not yet wired — returns `ok:false`)
- `ClosePosition` → `BrokerCmd::ClosePosition` (Alpaca) or `BrokerCmd::TastytradeClosePosition` (Tasty, from ADR-088)

Every dispatch replies via `web_msg_tx` with a `WebMsg::OrderResult { ok, message }` confirming which broker received the order. The host operator also sees a local log line mirroring every web-originated order so they can't miss them.

**New validation helpers in `web-protocol`:**
- `is_valid_order_side(&str) -> bool`
- `is_valid_order_type(&str) -> bool`
- `is_valid_order_qty(f64) -> bool` — finite, positive, ≤ `MAX_ORDER_QTY` (100,000)
- New constant `MAX_ORDER_QTY: f64`

## Tests

- **11 new EasyLanguage tests** (compiler crate)
- **12 new thinkScript tests** (compiler crate)
- **8 new web-protocol tests**: order_side_validation, order_type_validation, order_qty_validation, place_order_serde_roundtrip, place_order_limit_roundtrip, cancel_order_serde_roundtrip, close_position_serde_roundtrip, order_result_msg_roundtrip

**Total test count: 728** (up from 697)
- 131 mql5-compiler (+23)
- 497 engine
- 78 native
- 22 web-protocol (+8)

## Consequences

**Positive:**
- Community indicators published in EasyLanguage or thinkScript can now be compiled and run directly in TyphooN Terminal. The trader no longer needs to manually rewrite an indicator into MQL5 or PineScript first.
- Phone clients gain trade execution. Previously the WASM client was read-only (Phase 1 per ADR-073). Now the user can close a position from the train platform if the market moves against them.
- Four frontends on one IR proves the IR abstraction was the right call. Adding a fifth (Ninjascript, TS language, etc.) would follow the same pattern with minimal additional work.

**Trade-offs:**
- EasyLanguage and thinkScript frontends are line-based scanners, not full AST parsers. They handle the common community-indicator cases but will misparse anything exotic (nested if blocks, anonymous functions, multi-line conditional expressions). A future hard requirement for one of these would justify a proper pest grammar.
- Phone order entry trusts the web-server's passphrase + TLS for auth. There is no additional per-order confirmation step. If a phone is stolen with an active session, an attacker could place orders. Mitigation: the passphrase should be strong, and the user can reset it via Settings which invalidates existing sessions.
- Tastytrade cancel is not yet wired — returns `ok:false` with an explanatory message. Follow-up ADR if needed.
- Phone UI does not yet expose the order entry form — only the protocol exists. A follow-up will add a small form to `web/src/app.rs` so phone users can actually use the new commands. The protocol surface is ready.
- No new audio/sound library was added. Alert attention uses `ViewportCommand::RequestUserAttention` (from ADR-087) which is sufficient.

## Deferred / Out of Scope

- **Phase 2 indicators/drawing tools/MTF grid on phone** — each would significantly grow the WASM bundle (the existing client is 3.7 MB). Phase 1 UI remains intentionally minimal.
- **DARWIN analytics on phone** — requires porting the GPU DARWIN computations to a server-rendered preview. Out of scope.
- **Push notifications to phone** — requires a push service (FCM / APNS / WebPush). Out of scope.
- **Phone order form (HTML)** — wiring the new protocol commands into the WASM client UI. Deferred to a follow-up commit — the server + protocol are ready, so a later UI-only pass can land it in isolation.
- **EasyLanguage Buy/Sell trade signals** — would need a backtesting/paper-trading harness on the compiler runtime side. Separate feature.

## Related

- ADR-060 — MQL5 compiler pipeline (foundation for all frontends)
- ADR-069 — Feature gap list (originated the EasyLanguage/thinkScript asks)
- ADR-073 — WASM web client Phase 1 (originated the Phase 2 order entry deferral)
- ADR-088 — Closed ADR-069 HIGH/LOW gaps (context for the gap list being nearly empty)
