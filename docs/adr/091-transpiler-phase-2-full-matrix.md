# ADR-091 — Transpiler Phase 2: Full Cross-Language Matrix

**Status:** Implemented
**Date:** 2026-04-10

## Context

ADR-090 landed six new frontends and a Phase 1 cross-language transpiler
with 5 source languages (EL / TS / AFL / ProBuilder / Pine) × 4 targets
(MQL5 / Pine / EL / TS). Four languages could only be compile targets,
not transpile sources (MQL5, MQL4, NinjaScript, cAlgo), and five targets
were missing (MQL4, AFL, ProBuilder, NinjaScript, cAlgo).

This ADR closed the initial matrix, and follow-up ACSIL work expanded it:
**every supported language is now both a source and a target**. The current
result is a 10×10 = 100 directional capability.

## Decisions

### 1. Source-to-IR support for remaining 4 languages

Each was already building an `IrModule` internally but wrapping it in
`emit_wasm` before exposing it. The refactor extracts a `build_ir()`
helper that returns `(IrModule, IndicatorMeta)`, matching the pattern
already established for EasyLanguage / thinkScript / Pine / AFL /
ProBuilder.

| Language | Change |
|---|---|
| **MQL5** | New `build_mql5_ir(source) -> Result<(IrModule, IndicatorMeta), Vec<Diagnostic>>` in `lib.rs`. Wraps `parser::parse_mql5` → `ir::extract_metadata` → `ir::lower` into a single helper. `compile_mql5` is now a thin wrapper. |
| **MQL4** | Reuses `mql4::rewrite_mql4_to_mql5` then calls `build_mql5_ir`. Free. |
| **NinjaScript** | `ninjascript::build_ir(source)` extracted from `parse_ninjascript`. |
| **cAlgo** | `calgo::build_ir(source)` extracted from `parse_calgo`. |

### 2. Five new IR → source backends

| Target | Module | Description |
|---|---|---|
| **MQL4** | `transpile::emit_mql4` | `#property strict`, `extern` inputs, `init()`/`start()` entry points, `Close[i]`/`Open[i]` series access, `iMA(NULL,0,...)`/`iRSI(NULL,0,...)`/`iATR(NULL,0,...)`/`iStdDev(NULL,0,...)` built-ins. |
| **AFL** | `transpile::emit_afl` | `_SECTION_BEGIN/END`, `Param()` inputs, `EMA/MA/RSI/ATR/HHV/LLV/StDev/abs/sqrt/log` built-ins, `Plot(value, "label", color, style)` per buffer slot. |
| **ProBuilder** | `transpile::emit_probuilder` | `REM input ...` comments + local assignments, `ExponentialAverage[N]`/`Average[N]`/`RSI[N]`/`ATR[N]`/`Highest[N]`/`Lowest[N]`/`StdDev[N]` bracketed-length form, multi-return `RETURN ... AS "label", ... AS "label"`. |
| **NinjaScript** | `transpile::emit_ninjascript` | Complete C# class skeleton with `using` directives, `namespace`, `class : Indicator`, `OnStateChange()` with `AddPlot(...)`, `OnBarUpdate()` with `Values[N][0]` assignments, `[NinjaScriptProperty]`/`[Display]` attributes, `SMA(...)[0]`/`EMA(...)[0]`/`RSI(...)[0]`/`ATR(...)[0]`/`Math.*` built-ins. Input references are PascalCased to match the property declarations. |
| **cAlgo** | `transpile::emit_calgo` | Complete C# class skeleton with `[Indicator(Name=..., IsOverlay=...)]`, `[Parameter(..., DefaultValue=...)]` on typed properties, `[Output("label")]` on `IndicatorDataSeries`, `Calculate(int index)` with `Bars.ClosePrices[index]` long-form series access, `Indicators.SimpleMovingAverage(...).Result[index]`/`ExponentialMovingAverage`/`RelativeStrengthIndex`/`AverageTrueRange`/`StandardDeviation` built-ins. Input references PascalCased. |

### 3. C# identifier handling

Both C# backends now collect the lowercased IR input names and promote
`GetLocal` references to PascalCase when the referenced name matches an
input. This ensures `public int Period { get; set; }` in the property
block and `Period` in the method body match. Non-input locals (e.g.
`emaval`) remain lowercase as declared.

A new `pascal_case` helper handles all C# identifier emission: splits on
`_`, space, and `-`, uppercases each segment, drops non-alphanumeric
characters, and prefixes with `_` if the result starts with a digit (C#
identifier safety).

### 4. UI expansion

The "Transpile to:" dropdown in the Indicator Compiler window now lists
all 10 targets. The "Use as Source" button sets the source language to
match the transpile target (1:1 index mapping after this commit).

### 5. TargetLanguage enum expanded

`TargetLanguage` now has the same 10 variants as `SourceLanguage`:
`Mql5`, `Mql4`, `PineScript`, `EasyLanguage`, `ThinkScript`, `Afl`,
`ProBuilder`, `NinjaScript`, `Calgo`, `Acsil`.

## Tests

10 new transpile tests:
- `ninjascript_source_to_easylang_target` — NinjaScript → EL round-trip
- `calgo_source_to_mql5_target` — cAlgo → MQL5 round-trip
- `mql5_source_to_pine_target` — MQL5 source-to-IR path exercised
- `mql4_source_rewrites_and_transpiles` — MQL4 rewrite → IR → Pine
- `el_to_mql4_backend_emits_extern_and_init` — IR → MQL4 idioms
- `el_to_afl_backend_emits_section_and_plot` — IR → AFL idioms
- `el_to_probuilder_backend_emits_return` — IR → ProBuilder RETURN form
- `el_to_ninjascript_backend_emits_csharp_class` — IR → NinjaScript C#
- `el_to_calgo_backend_emits_indicator_attribute` — IR → cAlgo C#
- `full_matrix_smoke_test` — EL source → all 10 targets, non-empty output
- `pascal_case_helper` — helper correctness incl. leading-digit safety

**Historical workspace test count at ADR creation: 813** (up from 793 in
ADR-090). Subsequent frontend/backend follow-ups have raised
`mql5-compiler` to 229 unit tests.

- 229 mql5-compiler (+20 Phase 2 transpile/ACSIL follow-up, plus later O(1)
  and deferred-work regression tests)
- 497 engine
- 78 native
- 22 web-protocol

*Note: ACSIL (Sierra Chart) was added in a follow-up commit, expanding
the matrix to 10×10 = 100 directional conversions and adding 10 tests.*

## Full matrix

```
                ↓ Target
Source →    MQL5  MQL4  Pine  EL  TS  AFL  PB  Ninja  cAlgo  ACSIL
MQL5        ✅    ✅    ✅    ✅   ✅   ✅   ✅   ✅     ✅     ✅
MQL4        ✅    ✅    ✅    ✅   ✅   ✅   ✅   ✅     ✅     ✅
PineScript  ✅    ✅    ✅    ✅   ✅   ✅   ✅   ✅     ✅     ✅
EasyLanguage✅    ✅    ✅    ✅   ✅   ✅   ✅   ✅     ✅     ✅
thinkScript ✅    ✅    ✅    ✅   ✅   ✅   ✅   ✅     ✅     ✅
AFL         ✅    ✅    ✅    ✅   ✅   ✅   ✅   ✅     ✅     ✅
ProBuilder  ✅    ✅    ✅    ✅   ✅   ✅   ✅   ✅     ✅     ✅
NinjaScript ✅    ✅    ✅    ✅   ✅   ✅   ✅   ✅     ✅     ✅
cAlgo       ✅    ✅    ✅    ✅   ✅   ✅   ✅   ✅     ✅     ✅
ACSIL       ✅    ✅    ✅    ✅   ✅   ✅   ✅   ✅     ✅     ✅
```

## Consequences

### Positive

- The **full N×N cross-language transpiler is complete**. A trader can
  paste an indicator from any of the 10 supported platforms (MQL5, MQL4,
  PineScript, EasyLanguage, thinkScript, AFL, ProBuilder, NinjaScript,
  cAlgo, ACSIL) and get working source for any supported target in a
  single click. No other charting platform ships this.
- MQL5 source-to-IR round-trip now works, meaning MQL5 indicators can
  be converted to PineScript / EasyLanguage / thinkScript / etc. with no
  manual rewriting.
- C# backends produce idiomatic, compilable class skeletons that match
  the conventions of NinjaTrader 8 and cTrader respectively.

### Trade-offs

- **IR coverage remains the "80% indicator case"**: assignments, plot
  buffers, built-in TA calls, arithmetic, comparisons, statement-level `if`,
  and select/ternary lowering. Loop blocks, arrays, UDFs, and time-shifted
  series remain outside the shared transpiler subset. Indicators using those
  constructs may compile through source-specific paths, but cross-language
  output only preserves the shared subset.
- MQL5 source-to-IR depends on the pest grammar parsing the full source
  successfully. MQL5 source that hits unsupported grammar rules
  (templates, operator overloads, etc.) will fail gracefully with a
  diagnostic error rather than producing partial IR.

## Related

- ADR-090 — Multi-frontend expansion + Phase 1 transpiler (direct predecessor)
- ADR-060 — MQL5 compiler pipeline (foundation IR)
- ADR-089 — EasyLanguage + thinkScript compilers
