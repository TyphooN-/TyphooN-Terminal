# ADR-067: Multi-Frontend Expansion + Cross-Language Transpiler

**Status:** Implemented
**Date:** 2026-04-09

## Context

ADR-066 landed EasyLanguage and thinkScript compilers, bringing TyphooN
Terminal's indicator compiler to four languages: MQL5, PineScript v5,
EasyLanguage, and thinkScript. That ADR explicitly left the door open for
"a fifth (NinjaScript, TS language, etc.) would follow the same pattern."

The user's goal for this ADR is explicit:

> **Bring the largest amount of userbase + software to the platform via
> inherent compatibility.**

To do that we need to cover the remaining major community-indicator
languages, and — because all of our frontends already lower into a common
IR — we can additionally ship a feature no other trading platform offers:
**cross-language source transpilation**. A trader pastes an indicator
published in one dialect and gets clean, runnable source in their broker's
native dialect with a single click.

## Decisions

### 1. Six new frontends (all sharing the existing IR)

| Language | Crate module | Approach | LoC | Tests |
|---|---|---|---:|---:|
| **MQL4** (MetaTrader 4) | `mql5-compiler/src/mql4.rs` | Textual rewrite → reuses MQL5 parser | 420 | 11 |
| **AFL** (AmiBroker) | `mql5-compiler/src/afl.rs` | Line scanner → IR | 380 | 10 |
| **Pine v4** (TradingView) | extension to `pine.rs` | Auto-detect header, normalise bareword calls | +90 | +2 |
| **ProBuilder** (ProRealTime) | `mql5-compiler/src/probuilder.rs` | Line scanner → IR | 410 | 9 |
| **NinjaScript** (NinjaTrader) | `mql5-compiler/src/ninjascript.rs` | C# attribute scanner → IR (indicator subset) | 480 | 10 |
| **cAlgo** (cTrader) | `mql5-compiler/src/calgo.rs` | C# attribute scanner → IR (indicator subset) | 630 | 11 |

#### MQL4 — the biggest win
MT4 still has the **largest pool of retail algorithmic code in existence**
(two decades of EAs and indicators on ForexFactory / MQL5.com / Darwinex).
MQL4 and MQL5 share ~90% of their syntax. Rather than build a new parser
we do a **textual rewrite pass** that translates MQL4 idioms into their
MQL5 equivalents and then feeds the result through the existing pest
grammar:

| MQL4                                   | MQL5                                       |
|----------------------------------------|--------------------------------------------|
| `extern int Length = 14;`              | `input int Length = 14;`                   |
| `int init() { ... }`                   | `int OnInit() { ... }`                     |
| `int start() { ... }`                  | `int OnTick() { ... }`                     |
| `int deinit() { ... }`                 | `void OnDeinit(...) { ... }`               |
| `Bid` / `Ask`                          | `SymbolInfoDouble(_Symbol, SYMBOL_BID/ASK)` |
| `Close[i]` / `Open[i]` / `High[i]`     | `iClose(_Symbol, 0, i)` / ...              |
| `Bars` bareword                        | `iBars(_Symbol, 0)`                        |
| `Symbol()`                             | `_Symbol`                                  |
| `Digits` / `Point`                     | `_Digits` / `_Point`                       |

The rewrite pass is **string-and-comment-aware** — it tokenises each line
into alternating code/string/line-comment spans and only applies rewrites
to code spans. This means `Print("Bid is ", Bid);` correctly preserves the
string literal verbatim while still rewriting the bareword `Bid` argument.

`OrderSend(...)` and `OrderSelect(...)` cannot be ported textually (MQL5
uses `MqlTradeRequest`/`MqlTradeResult` structs with no 1:1 layout). We
emit a warning diagnostic and leave the call site untouched so the user
can port it manually. Indicators compile cleanly; EAs need a manual
trading-code port.

#### AFL — AmiBroker
20+ year legacy, one of the largest indicator archives on the web. Line
scanner in the same style as EasyLanguage/thinkScript. Supports:
`_SECTION_BEGIN("name")` → short name, `Param("label", default, …)` →
typed input, vector-oriented assignments, `Plot(value, "label", color)`,
and maps built-ins `EMA/SMA/RSI/ATR/HHV/LLV/StdDev/Abs/Sqrt/Log/Max/Min`
to the shared IR call names.

#### Pine v4 — extension to existing `pine.rs`
Pine v5 requires the `ta.` / `math.` namespace prefix on all function
calls. v4 uses bareword calls (`sma(close, 20)` instead of
`ta.sma(close, 20)`) and `study()` instead of `indicator()`. The parser
now auto-detects the version from `//@version=N` and runs a per-line
**prefix-aware rewrite** (`replace_unprefixed`) that turns bareword calls
into their v5 equivalents, but only when the token is NOT already
namespaced — so lines mixing both forms parse correctly.

#### ProBuilder — ProRealTime
Dominant European retail charting platform. BASIC-like syntax:

```
REM 20/50 EMA cross
ema20 = ExponentialAverage[20](close)
ema50 = ExponentialAverage[50](close)
c1 = ema20 CROSSES OVER ema50
RETURN c1 AS "Cross"
```

Supports `RETURN expr AS "label"` (including multi-return), bracketed-
length function syntax (`Average[14]`, `ExponentialAverage[14]`,
`RSI[14]`, `ATR[14]`, `Highest[14]`, `Lowest[14]`, `StdDev[14]`), the
`CROSSES OVER` / `CROSSES UNDER` binary operators, line-block
`IF ... THEN ... ELSE ... ENDIF`, and both `REM` and `//` comments.

#### NinjaScript — NinjaTrader (indicator subset)
NinjaScript is full C#; a real C# parser is a major undertaking. This
frontend handles the declarative portion common to community indicators:

- `[NinjaScriptProperty]` attribute pattern → typed inputs
- `AddPlot(Brushes.X, "Label")` calls inside `OnStateChange` → plot slots
- Assignment statements inside `OnBarUpdate()`: `Value[0] = expr;`,
  `Values[N][0] = expr;`, `SomePlot[0] = expr;`
- Built-in series `Close[0]` / `Open[0]` / … with trailing `[0]` stripping
- Built-in indicator calls: `SMA(src, period)[0]`, `EMA(...)[0]`,
  `RSI(...)[0]`, `ATR(period)[0]`, `MAX(...)`, `MIN(...)`, `StdDev(...)`
- `Math.Abs / Sqrt / Log / Max / Min(...)`
- `IsOverlay = false` → separate-window flag

Strategies (cBots), LINQ, nested `if`/`for`, and user classes are out of
scope. Community indicators almost always fit this subset.

#### cAlgo — cTrader (indicator subset)
Similar C# attribute pattern, but with cTrader's conventions:

- `[Indicator(Name = "...", IsOverlay = false)]` class-level attribute
  (short_name + overlay flag, scoped to the `[Indicator(...)]` balanced
  parens so we don't accidentally lift the name from a later
  `[Parameter]`)
- `[Parameter("Label", DefaultValue = N)]` on typed properties → inputs
- `[Output("Label")]` on `IndicatorDataSeries` properties → plot slots
- `Result[index] = expr;` and `MyOutputName[index] = expr;` assignments
- Built-in series: `Close`, `Open`, … (short form) AND `Bars.ClosePrices`,
  `MarketSeries.Close`, etc. (long form)
- Long-form indicator calls:
  `Indicators.SimpleMovingAverage(src, period).Result[index]`,
  `Indicators.ExponentialMovingAverage(...)`,
  `Indicators.RelativeStrengthIndex(...)`,
  `Indicators.AverageTrueRange(...)`,
  `Indicators.StandardDeviation(...)`
- Short-form calls (`SMA(Close, 20)`) also accepted
- `Math.Abs / Sqrt / Log / Max / Min(...)`
- `#region` / `#endregion` preprocessor directives are stripped

### 2. UI integration (Indicator Compiler window)

The language dropdown expands from 4 to **9 entries**:
MQL5 / MQL4 / PineScript / EasyLanguage / thinkScript / AFL / ProBuilder /
NinjaScript / cAlgo. File extensions the Load File... dialog now accepts:

| Language | Extensions |
|---|---|
| MQL5 | `.mq5`, `.mqh` |
| MQL4 | `.mq4`, `.mqh` |
| PineScript | `.pine` |
| EasyLanguage | `.el`, `.els` |
| thinkScript | `.ts`, `.tos` |
| AFL | `.afl` |
| ProBuilder | `.itf` |
| NinjaScript + cAlgo | `.cs` (disambiguated by content: `NinjaScriptProperty` keyword present → NinjaScript, else cAlgo) |

### 3. Cross-language transpiler (`mql5-compiler/src/transpile.rs`)

**This is the exclusive headline feature.** Because every frontend lowers
into the same `IrModule`, we can transpile source from any language to
any other by running `parse_X → IR → emit_Y`.

```rust
pub enum SourceLanguage { Mql5, Mql4, PineScript, EasyLanguage, ThinkScript,
                          Afl, ProBuilder, NinjaScript, Calgo }

pub enum TargetLanguage { Mql5, PineScript, EasyLanguage, ThinkScript }

pub fn transpile(source: &str, from: SourceLanguage, to: TargetLanguage)
    -> Result<String, String>;
```

#### Phase 1 coverage (this commit)

| Source ↓ → Target → | MQL5 | Pine v5 | EasyLanguage | thinkScript |
|---|:---:|:---:|:---:|:---:|
| EasyLanguage        | ✅ | ✅ | — | ✅ |
| thinkScript         | ✅ | ✅ | ✅ | — |
| AFL                 | ✅ | ✅ | ✅ | ✅ |
| ProBuilder          | ✅ | ✅ | ✅ | ✅ |
| Pine v4/v5          | ✅ | — | ✅ | ✅ |

Each backend emits idiomatic source:

- **MQL5**: complete `#property` header, `input int/double/bool`
  declarations, `double BufferN[];` buffer storage, `OnInit()` that calls
  `SetIndexBuffer(...)`, and a vectorised `OnCalculate(...)` loop. Function
  calls map to `iMA` / `iRSI` / `iATR` / `iStdDev` / `iHighest` / `iLowest`
  / `MathAbs` / `MathSqrt` / `MathLog` / `MathMax` / `MathMin`.
- **PineScript v5**: `//@version=5` header, `indicator(name, overlay=…)`
  declaration, `input.int/float/bool` inputs, `ta.sma / ta.ema / ta.rsi /
  ta.atr / ta.highest / ta.lowest / ta.stdev`, `math.*` helpers, and
  `plot(value, title="…", color=color.blue)` per buffer slot.
- **EasyLanguage**: `inputs:` / `variables:` blocks, `Average /
  XAverage / RSI / ATR / Highest / Lowest / StdDev / AbsValue /
  SquareRoot / Log / MaxList / MinList` built-ins, `<>` for
  inequality, `Plot1..N(value, "label")` per buffer slot.
- **thinkScript**: `declare lower;` when the source is a separate-window
  study, `input foo = default;` inputs, `def name = expr;` locals,
  `plot name = expr;` per buffer slot, `Average / ExpAverage / RSI / ATR /
  Highest / Lowest / StDev / AbsValue / Sqrt / Log / Max / Min` built-ins.

The `camel_case` / `snake_case` helpers map identifier conventions so the
output looks natural in the target language (`length` → `Length` in EL,
`Length` → `length` in Pine/TS).

#### UI integration
The Indicator Compiler window now has a **Transpile to: [dropdown]
[Transpile] [Use as Source] [Copy]** row directly beneath the Compile
button. Workflow:

1. Paste (or load) source in any supported language.
2. Set the source language (auto-detected from file extension).
3. Select a target language.
4. Click **Transpile**.
5. Review the output in the "Transpiled Output" panel.
6. Click **Use as Source** to immediately compile the transpiled output
   (the source language dropdown flips to the target), or **Copy** to
   paste it elsewhere.

Transpile errors appear in the Diagnostics list with a `TRANSPILE ERROR:`
prefix so they're easy to spot. A successful transpile adds an Info log
entry like `Transpiled EasyLanguage → Mql5: 42 lines`.

#### Phase 1 source-side limits (Phase 2 items)

The line-scanner frontends (EL / TS / AFL / ProBuilder / Pine) expose a
clean `build_ir(source) -> (IrModule, IndicatorMeta)` helper used by the
transpiler. The parser-based MQL5 frontend and the C#-flavoured
NinjaScript / cAlgo frontends currently only support **codegen output**,
not source-to-IR round-trip — transpiling *from* those languages will
return an error pointing at Phase 2. The workaround today is to use the
compile path for MQL5/NinjaScript/cAlgo (they still produce WASM) and
transpile between the five line-scanner languages freely.

### 4. Tests

- **11 new MQL4 tests** (rewrite correctness, string/comment safety, word
  boundaries, warning emission, end-to-end compile sanity)
- **10 new AFL tests** (simple plot, Param inputs, multi-plot, HHV/LLV
  mapping, block comments, case insensitivity, section name extraction; follow-up
  coverage adds `IIf` select lowering and O(1) duplicate-local suppression)
- **12 ProBuilder tests** (single / multi RETURN, REM + `//` comments,
  bracketed-length functions, CROSSES OVER/UNDER, ATR no-source form, line-block
  `IF ... THEN ... ELSE ... ENDIF`, duplicate-local suppression)
- **10 new NinjaScript tests** (property parsing, multi-plot, IsOverlay
  flag, comment stripping, `Math.Abs` mapping, `SMA` mapping; follow-up adds
  duplicate-local suppression)
- **11 new cAlgo tests** (Parameter extraction, Output binding, IsOverlay,
  multi-output, long+short series forms, `[Indicator(Name=…)]` extraction,
  comment + `#region` stripping; follow-up adds duplicate-local suppression)
- **2 new Pine v4 tests** (v4 header + bareword `sma/rsi/study` rewrite,
  `replace_unprefixed` regression guard)
- **10 new transpile tests** (EL→MQL5, EL→Pine, TS→EL, Pine→TS, AFL→MQL5,
  ProBuilder→EL, Pine→EL, unsupported-source error path, `camel_case`
  helper, EL→MQL5 `math_abs` mapping)

**Historical baseline at ADR creation: 793 workspace tests** (up from 728 in
ADR-066). Follow-up compiler frontend comb-overs have since raised
`mql5-compiler` coverage to 229 unit tests.

- 196 mql5-compiler (+65)
- 497 engine
- 78 native
- 22 web-protocol

## Consequences

### Positive

- **Compatibility coverage now includes the ten-language matrix.** MQL5,
  MQL4, PineScript, EasyLanguage, thinkScript, AFL, ProBuilder, NinjaScript,
  cAlgo, and Sierra Chart ACSIL all have source-to-IR and target emission
  paths for the shared indicator subset.
- **MQL4 unlocks the largest single pool of retail algorithmic code
  ever published.** Two decades of forum-posted EAs and indicators are
  now one load-file click away from running on the TyphooN Terminal.
- **The cross-language transpiler is a genuinely unique feature.** No
  other charting platform I'm aware of lets you paste a PineScript
  indicator and get working MQL5 (or vice versa). Because it's built on
  the shared IR, adding any new source/target combination is O(1)
  backend work, not O(N²).
- **The line-scanner pattern has now been proven on five languages**
  (EL, TS, AFL, ProBuilder, plus Pine via the normalise-and-scan
  approach). Adding a sixth is purely mechanical.
- **C# frontends (NinjaScript, cAlgo) use attribute-scanning rather
  than parsing** — a pragmatic shortcut that handles the 90% case
  without dragging in a real C# parser.

### Trade-offs

- **Source-to-IR round-trip is implemented for all ten listed languages.**
  MQL4 rewrites through MQL5, C#-style platforms use pragmatic attribute
  scanners, and line-scanner languages feed the same shared IR. The trade-off
  is still subset coverage, not direction-matrix coverage.
- **IR coverage beyond the initial ADR:** the shared IR now represents
  statement-level `if` blocks and the WASM codegen handles ternary/select
  lowering through `__select_f64`. AFL `IIf(cond, a, b)` and ProBuilder
  `IF ... THEN ... ELSE ... ENDIF` line-blocks are implemented follow-ups.
- **Remaining IR gaps:** loops (`for`/`while` in line-scanner frontends),
  arrays, user-defined functions, time-shifted series access (`close[5]`,
  `Close[5]`, `Close.Last(5)`), and trade-signal operations are still outside
  the common indicator IR subset. Indicators using those constructs may compile
  through a source-specific path, but transpiler output only preserves the
  shared subset.
- **C# frontend brittleness.** NinjaScript and cAlgo attribute scanning
  will fall over on exotic patterns like `[DataSeries(Output =
  ResultType.Line)]` attributes split across multiple lines, or
  `#nullable` annotations. Community-published indicators almost always
  follow the template conventions, so this is a real-world safe
  trade-off.
- **No trade-signal transpiling.** `Buy` / `Sell` statements in AFL,
  `EnterLong` in NinjaScript, `Robot`-derived cBots in cAlgo,
  `OrderSend` in MQL4 — none of these cross-map cleanly between
  platforms because the underlying broker APIs have different semantics
  (single-position vs netted, OCO support, etc.). All frontends are
  indicator-only.

## Deferred Follow-up Context

### Phase 2 — transpile sources / additional targets
- **Resolved:** MQL5/MQL4 source-to-IR, NinjaScript/cAlgo source wrappers,
  ACSIL source support, and IR emitters for MQL4/AFL/ProBuilder/NinjaScript/
  cAlgo/ACSIL are implemented. The active follow-up area is no longer matrix
  plumbing; it is deeper shared-IR coverage for constructs that do not fit the
  common indicator subset.

### Phase 2 — Remaining IR coverage
- **Loop control in line-scanner frontends**: `for`/`while` blocks remain
  source-specific or skipped outside the full MQL parser.
- **Time-shifted series access**: `close[5]`, `Close[5]`, `Close.Last(5)`
  — need an IR `PrevBar(n, expr)` node.
- **Arrays + user-defined functions**: requires a proper symbol table.
- **Additional source-language select syntax**: AFL `IIf(cond, a, b)` and
  MQL-style ternary/select lowering are implemented; any remaining language
  aliases such as `iff(...)` can map onto the same select primitive.
- **Trade-signal IR**: `Buy` / `Sell` / `EnterLong` — new IR ops that
  lower to broker-specific calls per target language.

### Out of scope (permanently) — ~~subsequently implemented~~
- ~~**Sierra Chart ACSIL** — C++ with custom preprocessor. Users of this
  platform are already systems programmers; the compatibility argument
  doesn't apply.~~ **Implemented in follow-up commit** — ACSIL landed as
  the 10th frontend with full transpiler support (10×10 matrix). See the
  ACSIL commit for details.
- **Wealth-Lab / TradersStudio / MetaStock Formula Language** — declining
  platforms with small userbases. If the community demand materialises we
  can add them later, but they're not on the near-term radar.
- **EFS (eSignal JavaScript)** — the platform is in long-term decline
  and a JS subset parser is expensive relative to the shrinking userbase.

## Related

- ADR-040 — MQL5 compiler pipeline (the common IR + WASM/WGSL codegen)
- ADR-047 — Feature gap list (originated the EasyLanguage / thinkScript /
  MQL4 / NinjaScript asks)
- ADR-066 — EasyLanguage + thinkScript compilers (directly preceding ADR;
  this one picks up the "adding a fifth frontend" hint and takes it to six)

## Consolidated execution-log ADRs (2026-06)

- **ADR-068** — Transpiler Phase 2 (full cross-language matrix): the phase-2 pass
  that completed the frontend matrix begun here. Now a stub; detail in git history.
