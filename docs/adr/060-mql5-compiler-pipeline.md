# ADR-060: MQL5 Compiler Pipeline — Source to GPU/CPU Execution

**Status:** Phase 1 Implemented (WASM), Phase 2 Implemented (WGSL), Phase 3 Implemented (PineScript) | **Date:** 2026-03-27 | **Updated:** 2026-04-03

## Context

TyphooN Terminal includes an MQL5 indicator compiler that can parse `.mq5`/`.mqh` files and compile them for execution. This enables users to run custom indicators without MetaTrader 5.

## Architecture

```
MQL5 Source (.mq5/.mqh)
    │
    ▼
Parser (pest grammar → AST)
    │  - Handles: #property, #include, #ifdef __MQL5__/__MQL4__
    │  - Supports: OnCalculate, OnInit, iSeries, SetIndexBuffer
    │  - Types: int, double, bool, string, color, enums, structs
    │
    ▼
IR (Intermediate Representation)
    │  - Target-agnostic lowered form
    │  - Typed, resolved buffer indices, explicit draw commands
    │  - IrModule { buffers, inputs, functions, on_calculate, globals }
    │
    ├──────────────────┐
    ▼                  ▼
WASM Backend       WGSL Backend
(Phase 1 ✓)        (Phase 2 ✓)
    │                  │
    ▼                  ▼
CPU Execution      GPU Execution
via wasmtime       via wgpu compute shaders
```

## Phase 1: WASM Backend (Implemented)

The `emit_wasm()` codegen produces a valid WebAssembly module:
- Imports: `iOpen`, `iHigh`, `iLow`, `iClose`, `iVolume`, `iBars`, math functions
- Exports: `on_calculate(rates_total, prev_calculated) -> i32`
- Memory: shared linear memory for indicator buffers
- Runtime: Rust host provides bar data through imports

**Status:** Parser handles core MQL5 syntax. Codegen produces working WASM. Runtime executes with bar data from SQLite cache.

## Phase 2: WGSL Backend (Implemented)

The `emit_wgsl()` codegen (1100 lines) produces valid WGSL compute shaders:
- Each indicator buffer becomes a `storage` binding
- OnCalculate loop maps to `@compute @workgroup_size(256)` main function
- iSeries functions map to bar data buffer reads (iOpen/iHigh/iLow/iClose/iVolume)
- Math functions use WGSL built-ins (`sqrt`, `log`, `abs`, `max`, `min`)
- Input parameters gathered into a `Params` uniform struct with `bar_count`
- Ternary expressions compiled to WGSL `select()`
- 26 WGSL-specific tests covering codegen output

**Parser bug fix (postfix_op unwrapping):** `postfix_op` rule now correctly distinguishes between `++`/`--` operators (which ARE the operator) and wrapped `call_args`/`index_access`/`member_access` (which contain an inner child). Previously unwrapping `postfix_op` unconditionally caused panics on increment/decrement expressions.

**Status:** Same MQL5 source compiles to both CPU (WASM) and GPU (WGSL). 75 compiler tests passing (parser + WASM codegen + WGSL codegen).

**Constraints:**
- WGSL has no recursion, limited control flow
- Sequential indicators (EMA, KAMA) need scan/prefix-sum patterns
- Not all MQL5 constructs map to GPU (file I/O, network, GlobalVariables)

## Grammar Coverage

```
✓ Preprocessor (#include, #define, #ifdef)
✓ #property directives
✓ Type system (int, double, bool, string, color, enum, struct, class)
✓ Functions (OnInit, OnCalculate, OnDeinit, custom)
✓ Control flow (if/else, for, while, switch)
✓ Expressions (arithmetic, comparison, logical, ternary)
✓ Arrays (static, dynamic)
✓ iSeries functions (iOpen, iHigh, iLow, iClose, iVolume, iBars, iTime)
✓ Indicator buffers (SetIndexBuffer, SetIndexStyle)
✓ Math functions (MathSqrt, MathLog, MathAbs, MathMax, MathMin)
✗ Trading functions (OrderSend, etc.) — intentionally excluded
✗ File I/O — not applicable for indicators
✗ Network — not applicable for indicators
```

## Phase 3: PineScript v5 Parser (Implemented)

`pine.rs` parses PineScript v5 source into the same IR used by the MQL5 pipeline:

- `//@version=5` header, `indicator()` declaration
- `input.int()`, `input.float()`, `input.bool()`, `input.string()` → `IrInput`
- `ta.sma()`, `ta.ema()`, `ta.rsi()`, `ta.atr()`, `ta.highest()`, `ta.lowest()` → `IrExpr::Call`
- `ta.crossover()`, `ta.crossunder()`, `ta.stdev()`, `ta.change()`, `ta.tr`, `nz()`
- `plot()` with title/color extraction → `PlotDef` + `SetBuffer`
- `math.abs()`, `math.sqrt()`, `math.log()`, `math.max()`, `math.min()`
- Built-in series: `close`, `open`, `high`, `low`, `volume`, `bar_index`
- Variable assignments, `var` declarations, binary operators
- Compiled to WASM via same `emit_wasm()` codegen as MQL5

**Status:** 7 PineScript tests + 75 MQL5 tests = 82 total tests passing. Covers common TradingView indicators (SMA, EMA, RSI, ATR, multi-plot). Wired via `compile_pine()` in `lib.rs`.

## Consequences

- **Pro:** Users can run MQL5 indicators without MetaTrader
- **Pro:** Users can run TradingView PineScript indicators natively
- **Pro:** GPU compilation path provides massive speedup (both backends implemented)
- **Pro:** Same source compiles to CPU (WASM) and GPU (WGSL) from single AST
- **Pro:** PineScript reuses same IR → WASM pipeline as MQL5
- **Pro:** Sandboxed execution prevents malicious indicator code
- **Pro:** 82 tests covering parser, WASM codegen, WGSL codegen, and PineScript
- **Con:** Not 100% MQL5 compatible (trading functions excluded)
- **Con:** PineScript subset — covers common indicators, not full language
- **Con:** GPU path limited to parallelizable indicators
