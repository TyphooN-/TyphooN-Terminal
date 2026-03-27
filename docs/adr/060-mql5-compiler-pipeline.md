# ADR-060: MQL5 Compiler Pipeline — Source to GPU/CPU Execution

**Status:** Phase 1 Implemented (WASM), Phase 2 Proposed (WGSL) | **Date:** 2026-03-27

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
WASM Backend       WGSL Backend (Proposed)
(Phase 1 ✓)        (Phase 2 — Future)
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

## Phase 2: WGSL Backend (Proposed)

Add `emit_wgsl()` to generate GPU compute shaders:
- Each indicator buffer becomes a GPU storage buffer
- OnCalculate loop parallelized across workgroups
- iSeries functions map to buffer reads
- Math functions use WGSL built-ins

**Benefits:**
- Custom MQL5 indicators run on GPU automatically
- Same source compiles to both CPU (WASM) and GPU (WGSL)
- 100-1000× speedup for indicators on large datasets

**Challenges:**
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

## Consequences

- **Pro:** Users can run MQL5 indicators without MetaTrader
- **Pro:** GPU compilation path provides massive speedup
- **Pro:** Same source works on CPU (WASM) and GPU (WGSL)
- **Pro:** Sandboxed execution prevents malicious indicator code
- **Con:** Not 100% MQL5 compatible (trading functions excluded)
- **Con:** GPU path limited to parallelizable indicators
