# ADR-047: MQL5 & PineScript Compatibility Layer

**Status:** Stage 1-3 Implemented (Parser + IR + Codegen + Tauri + Frontend)
**Date:** 2026-03-24

## Context

TyphooN Terminal's primary advantage is local rendering + multi-broker data. But MT5's killer feature is its ecosystem: thousands of EAs and custom indicators written in MQL5. TradingView's equivalent is PineScript. If we can run MQL5 indicators/EAs and PineScript indicators natively, we inherit both ecosystems instantly.

The goal: support 99.99% of MQL5 and PineScript functionality so users can compile their existing code directly in TyphooN Terminal. This creates a massive competitive moat — no other terminal runs both.

## Architecture

### Compiler Pipeline

```
MQL5 source (.mq5)     PineScript source (.pine)
       │                         │
       ▼                         ▼
  MQL5 Parser (Rust)      Pine Parser (Rust)
       │                         │
       ▼                         ▼
  ┌──────────────────────────────────┐
  │    TyphooN IR (Intermediate      │
  │    Representation)               │
  │    - Typed AST with bar access   │
  │    - Buffer declarations         │
  │    - Plot/draw commands          │
  │    - Order/position commands     │
  └──────────────────────────────────┘
       │                    │
       ▼                    ▼
   WASM Backend        JS Backend
   (indicators)        (EA logic)
       │                    │
       ▼                    ▼
   GPU Renderer        Tauri Commands
   (60fps lines,       (order placement,
    histograms,         position mgmt)
    fills, markers)
```

### Why Compile to WASM (Not Interpret)

1. **Performance**: WASM runs at near-native speed. Fisher Transform on 50K bars: ~2ms WASM vs ~200ms interpreted
2. **GPU Integration**: WASM indicators output Float64Arrays directly to GPU `add_line()`/`add_histogram()`
3. **Sandboxing**: WASM has no filesystem/network access by default — safe to run untrusted code
4. **Async**: WASM computation runs in Web Workers, never blocks main thread
5. **Size**: Compiled indicators are 2-10KB each (vs megabytes for an interpreter)

### Phase 1: MQL5 Indicator Support

#### MQL5 API Surface to Support

**Core indicator functions** (most-used, covers 95% of indicators):

```rust
// Bar data access
iOpen(symbol, timeframe, shift) -> f64
iHigh(symbol, timeframe, shift) -> f64
iLow(symbol, timeframe, shift) -> f64
iClose(symbol, timeframe, shift) -> f64
iVolume(symbol, timeframe, shift) -> f64
iTime(symbol, timeframe, shift) -> datetime
iBars(symbol, timeframe) -> i32
iBarShift(symbol, timeframe, time) -> i32

// Built-in indicators
iMA(symbol, tf, period, shift, method, price) -> f64
iRSI(symbol, tf, period, shift, price) -> f64
iATR(symbol, tf, period, shift) -> f64
iMACD(symbol, tf, fast, slow, signal, shift, price, buffer) -> f64
iStochastic(symbol, tf, k, d, slow, method, shift, buffer) -> f64
iBands(symbol, tf, period, dev, shift, price, buffer) -> f64
iCCI(symbol, tf, period, shift, price) -> f64
iADX(symbol, tf, period, shift, buffer) -> f64
// ... full set of ~35 built-in indicator functions

// Array functions
ArrayMaximum(array, count, start) -> i32
ArrayMinimum(array, count, start) -> i32
ArrayCopy(dst, src, dst_start, src_start, count)
ArrayResize(array, size) -> i32
ArraySetAsSeries(array, flag)

// Math
MathMax, MathMin, MathAbs, MathSqrt, MathPow, MathLog, MathExp
MathFloor, MathCeil, MathRound
MathSin, MathCos, MathTan, MathArctan

// Drawing
SetIndexBuffer(index, array, type)  // INDICATOR_DATA, INDICATOR_COLOR_INDEX
SetIndexStyle(index, type, style, width, color)  // DRAW_LINE, DRAW_HISTOGRAM, etc.
PlotIndexSetInteger(index, prop, value)
PlotIndexSetDouble(index, prop, value)
PlotIndexSetString(index, prop, value)
IndicatorSetInteger(prop, value)  // INDICATOR_DIGITS, etc.
IndicatorSetString(prop, value)  // INDICATOR_SHORTNAME

// Object creation (horizontal lines, rectangles, text labels)
ObjectCreate(name, type, window, time1, price1, ...)
ObjectSetInteger(name, prop, value)
ObjectSetDouble(name, prop, value)
ObjectSetString(name, prop, value)
ObjectDelete(name)

// String/conversion
StringFormat, IntegerToString, DoubleToString
StringFind, StringSubstr, StringLen
StringToInteger, StringToDouble

// Time
TimeCurrent() -> datetime
TimeLocal() -> datetime
TimeToStruct(datetime) -> MqlDateTime
TimeGMT() -> datetime
```

**Draw types to support:**

| MQL5 Draw Type | GPU Mapping |
|---|---|
| DRAW_LINE | `gpu.add_line()` |
| DRAW_SECTION | `gpu.add_line()` (segments) |
| DRAW_HISTOGRAM | `gpu.add_histogram()` |
| DRAW_HISTOGRAM2 | `gpu.add_histogram()` (dual) |
| DRAW_ARROW | GPU markers (new) |
| DRAW_ZIGZAG | `gpu.add_line()` (sparse) |
| DRAW_FILLING | `gpu.add_fill()` |
| DRAW_BARS | `gpu.set_data()` (OHLC bars mode) |
| DRAW_CANDLES | `gpu.set_data()` (candle mode) |
| DRAW_COLOR_LINE | `gpu.add_line()` per segment |
| DRAW_COLOR_HISTOGRAM | `gpu.add_histogram()` with per-bar colors |
| DRAW_NONE | (skip) |

#### MQL5 Indicator Compilation Flow

```
user_indicator.mq5
       │
       ▼ (Rust parser: pest/nom)
  MQL5 AST
       │
       ▼ (Type checker + IR lowering)
  TyphooN IR
       │
       ▼ (WASM code generator)
  user_indicator.wasm (2-10KB)
       │
       ▼ (Runtime loads in Web Worker)
  Worker: calls OnCalculate() with bar data
       │
       ▼ (Returns Float64Array buffers)
  Main thread: gpu.add_line(), gpu.add_histogram(), etc.
```

### Phase 2: MQL5 EA (Expert Advisor) Support

EAs are more complex — they need order management, position tracking, and event handling.

**EA API surface:**

```rust
// Event handlers (compiled to async Rust/JS)
OnInit() -> int
OnDeinit(reason)
OnTick()
OnTimer()
OnChartEvent(id, lparam, dparam, sparam)

// Trading functions (route to Tauri commands → Alpaca/Darwinex)
OrderSend(request: MqlTradeRequest) -> bool
OrderModify(ticket, price, sl, tp, expiration) -> bool
OrderDelete(ticket) -> bool
PositionClose(ticket) -> bool
PositionModify(ticket, sl, tp) -> bool

// Position/order queries
PositionsTotal() -> int
PositionGetInteger(prop) -> long
PositionGetDouble(prop) -> double
PositionGetString(prop) -> string
OrdersTotal() -> int
OrderGetInteger(prop) -> long
OrderGetDouble(prop) -> double

// Account info
AccountInfoDouble(prop) -> double  // ACCOUNT_BALANCE, ACCOUNT_EQUITY, etc.
AccountInfoInteger(prop) -> long
AccountInfoString(prop) -> string

// Symbol info
SymbolInfoDouble(symbol, prop) -> double  // SYMBOL_BID, SYMBOL_ASK, SYMBOL_POINT, etc.
SymbolInfoInteger(symbol, prop) -> long
SymbolInfoString(symbol, prop) -> string
MarketInfo(symbol, prop) -> double  // legacy

// Alert/notification
Alert(message)
Print(message)
Comment(message)
SendNotification(message)
```

**Broker abstraction**: MQL5 `OrderSend()` maps to TyphooN's existing `submit_order` Tauri command. The EA runtime translates `MqlTradeRequest` → Alpaca/Darwinex order format.

### Phase 3: PineScript Indicator Support

PineScript is simpler than MQL5 — it's a declarative language focused on indicators.

**PineScript API surface:**

```
// Series functions
ta.sma(source, length)
ta.ema(source, length)
ta.rsi(source, length)
ta.macd(source, fast, slow, signal)
ta.atr(length)
ta.stoch(close, high, low, length)
ta.cci(source, length)
ta.bb(source, length, mult)
ta.highest(source, length)
ta.lowest(source, length)
ta.crossover(a, b)
ta.crossunder(a, b)
ta.change(source, length)
ta.roc(source, length)
ta.pivothigh(source, leftbars, rightbars)
ta.pivotlow(source, leftbars, rightbars)

// Plotting
plot(series, title, color, linewidth, style)
plotshape(series, title, location, color, style, text)
plotchar(series, title, char, location, color)
hline(price, title, color, linestyle)
fill(plot1, plot2, color)
bgcolor(color)
barcolor(color)

// Input
input.int(defval, title, minval, maxval)
input.float(defval, title, minval, maxval)
input.bool(defval, title)
input.string(defval, title, options)
input.source(defval, title)
input.color(defval, title)

// Math
math.abs, math.max, math.min, math.sqrt, math.pow, math.log
math.round, math.floor, math.ceil
math.sign, math.avg

// String
str.tostring, str.format, str.contains, str.length

// Built-in variables
open, high, low, close, volume, time, bar_index
na, true, false
```

**PineScript compilation**: PineScript → TyphooN IR → WASM. PineScript's series semantics map naturally to our bar-indexed buffer model.

### Phase 4: User Interface

#### Indicator/EA Manager Panel

```
┌─ Indicators ────────────────────────────┐
│ ⬛ NNFX SYSTEM                          │
│   ☑ MultiKAMA (14/2/30)     [compiled] │
│   ☑ Fisher (32)              [compiled] │
│   ☑ ATR Projection (14)     [compiled] │
│   ☑ Supply/Demand            [compiled] │
│                                         │
│ ⬛ USER INDICATORS                      │
│   ☑ MyRSIDivergence.mq5     [compiled] │
│   ☐ CustomMACD.pine          [compiled] │
│   ⚠ BrokenIndicator.mq5     [error]    │
│                                         │
│ [+ Add MQL5] [+ Add Pine] [+ Add WASM] │
└─────────────────────────────────────────┘
```

#### Compilation Workflow

1. User clicks "+ Add MQL5" → file picker opens
2. MQL5 source is parsed + compiled to WASM (Rust backend, <100ms for typical indicator)
3. Compiled WASM stored in `kv_cache` with source hash
4. Indicator appears in panel with ☑ checkbox
5. Checking the box loads WASM in Worker → computes → renders on GPU

#### Error Reporting

```
⚠ BrokenIndicator.mq5 — Compilation failed:
  Line 42: Unknown function 'iCustom' — not yet supported
  Line 67: Type mismatch: expected 'double', got 'string'
```

### Implementation Plan

#### Stage 1: MQL5 Indicator Parser (Rust)
- New crate: `mql5-compiler/` in workspace
- Parser: `pest` grammar for MQL5 syntax (C-like, well-documented spec)
- Type checker: resolve indicator buffers, iSeries calls, math functions
- IR lowering: MQL5 AST → TyphooN IR

#### Stage 2: WASM Code Generator
- IR → WASM bytecode (using `walrus` or `wasm-encoder` crate)
- Runtime shim: provides `iOpen()`, `iHigh()`, etc. as WASM imports
- Each compiled indicator is a standalone `.wasm` file (2-10KB)

#### Stage 3: Indicator Runtime (Frontend)
- Load compiled WASM in Web Worker
- Call `OnCalculate()` with bar data buffers
- Receive output buffers → route to GPU `add_line()`/`add_histogram()`/`add_fill()`
- Support `#property indicator_separate_window` → GPU sub-pane

#### Stage 4: PineScript Parser
- Extend the same pipeline: Pine parser → TyphooN IR → WASM
- PineScript is simpler — fewer constructs, no pointers, no classes
- Series semantics map directly to bar-indexed buffers

#### Stage 5: EA Runtime
- EA compilation: MQL5 EA → WASM + Rust event loop
- `OnTick()` called on each price update (from Alpaca WS / MT5 sync)
- Trading functions route through Tauri commands to broker APIs
- Backtesting: replay historical bars through EA `OnTick()` in fast-forward

### Compatibility Notes

#### What We Won't Support (0.01%)
- `#import` DLL calls (security risk, platform-specific)
- `iCustom()` calling other MT5 indicators (they'd need to be compiled too)
- `ResourceCreate()`/`ResourceReadImage()` (bitmap resources)
- `WebRequest()` (network access from indicator — security sandbox)
- MQL5 OOP inheritance with virtual methods (rare in indicators, needed for some EAs)
- `EventSetTimer()` with sub-second resolution (browser limitation)

#### What Maps Cleanly
- All `iSeries()` functions → direct bar data access
- All `Array*()` functions → Rust Vec / WASM linear memory
- All `Math*()` functions → WASM math intrinsics (hardware-accelerated)
- All `DRAW_*` types → GPU rendering pipeline
- `OrderSend()` → Tauri `submit_order` command
- `PositionGet*()` → Tauri `get_positions` command
- `Alert()` → Desktop notification via Tauri

### Dependencies

**No new external dependencies** for the parser — Rust's `pest` or `nom` handles the grammar.
WASM generation uses `wasm-encoder` (pure Rust, no LLVM dependency).

### Performance Characteristics

| Operation | Expected Time |
|-----------|--------------|
| MQL5 parse + compile (typical indicator) | <100ms |
| PineScript parse + compile | <50ms |
| Indicator OnCalculate (50K bars, WASM) | 1-5ms |
| GPU render (lines + histograms) | <1ms |
| EA OnTick() (single tick) | <0.1ms |

### Stage 6: Built-in IDE

TyphooN Terminal includes an integrated code editor for MQL5 and PineScript development.

#### IDE Features
- **Monaco Editor** (VS Code's editor, MIT licensed, 2MB) embedded in a Tauri window
- **MQL5 syntax highlighting** — keyword/type/function coloring, bracket matching
- **PineScript syntax highlighting** — series/plot/ta.* function coloring
- **Autocomplete** — context-aware: `iOpen(`, `ta.sma(`, `OrderSend(` with parameter hints
- **Real-time error markers** — red squiggles on parse errors, yellow on warnings
- **Compile button** — triggers Rust backend parser → WASM, shows errors inline
- **Live preview** — compiled indicator renders on active chart immediately
- **Input parameter panel** — auto-generated from `input.int()`/`#property` declarations
- **File tree** — `~/.config/typhoon-terminal/scripts/` organized by type (indicators/EAs/libraries)

#### IDE Architecture
```
┌─ IDE Window (Tauri) ────────────────────┐
│ ┌─ File Tree ─┐ ┌─ Monaco Editor ─────┐ │
│ │ indicators/ │ │ // @version=5       │ │
│ │  MyRSI.mq5  │ │ indicator("MyRSI")  │ │
│ │  KAMA.pine   │ │ rsi = ta.rsi(c, 14) │ │
│ │ EAs/         │ │ plot(rsi)           │ │
│ │  TyphooN.mq5 │ │                     │ │
│ └──────────────┘ └─────────────────────┘ │
│ ┌─ Output / Errors ────────────────────┐ │
│ │ ✓ Compiled to WASM (3.2KB, 42ms)    │ │
│ │ ⚠ Line 12: unused variable 'x'      │ │
│ └──────────────────────────────────────┘ │
│ [Compile] [Apply to Chart] [Backtest]    │
└──────────────────────────────────────────┘
```

#### Keyboard Shortcuts
| Shortcut | Action |
|---|---|
| Ctrl+S | Compile + apply to chart |
| Ctrl+B | Compile only (check errors) |
| Ctrl+Shift+B | Compile + backtest |
| F5 | Run EA in live mode |
| F8 | Step through EA (debug mode) |

#### Implementation
- Monaco Editor loaded via CDN or bundled (tree-shaken to ~800KB)
- Tauri window: `openIdeWindow(filePath)` — new window with editor
- Compile: `invoke("compile_mql5", { source })` → returns `{ wasm: Uint8Array, errors: [] }`
- Apply: load compiled WASM in indicator worker → GPU render

### Security Model

- Compiled WASM runs in Web Worker sandbox (no DOM, no network, no filesystem)
- EA trading functions require explicit user authorization per EA
- Source code stored locally — never uploaded anywhere
- Compilation happens locally in Rust backend — no cloud dependency
