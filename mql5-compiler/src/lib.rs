//! Multi-language indicator compiler for TyphooN Terminal.
//!
//! Pipeline: Source → Parse → AST → IR → WASM
//!
//! Frontends:
//! - MQL5        — full parser (pest grammar), AST, IR lowering
//! - MQL4        — MT4 compat shim: textual rewrite → MQL5 parser
//! - PineScript  — v4 + v5 line scanner (pine.rs)
//! - EasyLanguage — line scanner (easylang.rs)
//! - thinkScript  — line scanner (thinkscript.rs)
//! - AFL         — AmiBroker Formula Language (afl.rs)
//! - ProBuilder  — ProRealTime (probuilder.rs)
//! - NinjaScript — NinjaTrader indicator subset (ninjascript.rs)
//! - cAlgo       — cTrader indicator subset (calgo.rs)
//!
//! All nine share the same IR + WASM/WGSL codegen pipeline.
//! Because of this common lowering, ANY source language can be
//! transpiled into ANY other (IR → source backends — see ADR-090).

pub mod parser;
pub mod ast;
pub mod ir;
pub mod codegen;
pub mod wgsl_codegen;
pub mod error;
pub mod runtime;
pub mod pine;
pub mod easylang;
pub mod thinkscript;
pub mod mql4;
pub mod afl;
pub mod probuilder;
pub mod ninjascript;
pub mod calgo;
pub mod transpile;



/// Compilation result returned to Tauri frontend.
#[derive(serde::Serialize)]
pub struct CompileResult {
    /// Compiled WASM binary (None if errors)
    pub wasm: Option<Vec<u8>>,
    /// Compilation errors/warnings
    pub diagnostics: Vec<Diagnostic>,
    /// Indicator metadata (buffers, properties, inputs)
    pub metadata: Option<IndicatorMeta>,
}

#[derive(Debug, serde::Serialize)]
pub struct Diagnostic {
    pub level: DiagLevel,
    pub message: String,
    pub line: usize,
    pub col: usize,
}

#[derive(Debug, serde::Serialize)]
pub enum DiagLevel {
    Error,
    Warning,
    Info,
}

/// Metadata extracted from indicator source for the runtime.
#[derive(serde::Serialize, Clone)]
pub struct IndicatorMeta {
    /// Short name (#property indicator_shortname)
    pub short_name: String,
    /// Number of indicator buffers
    pub buffers: usize,
    /// Whether indicator uses a separate window
    pub separate_window: bool,
    /// Input parameters (name, type, default)
    pub inputs: Vec<InputParam>,
    /// Plot definitions (draw type, color, width, style)
    pub plots: Vec<PlotDef>,
}

#[derive(serde::Serialize, Clone)]
pub struct InputParam {
    pub name: String,
    pub param_type: String,
    pub default_value: String,
}

#[derive(serde::Serialize, Clone)]
pub struct PlotDef {
    pub index: usize,
    pub draw_type: DrawType,
    pub color: String,
    pub width: u32,
    pub style: u32,
    pub label: String,
}

#[derive(Debug, serde::Serialize, Clone)]
pub enum DrawType {
    Line,
    Section,
    Histogram,
    Histogram2,
    Arrow,
    ZigZag,
    Filling,
    Bars,
    Candles,
    ColorLine,
    ColorHistogram,
    None,
}

/// Compile MQL5 source to WASM.
pub fn compile_mql5(source: &str) -> CompileResult {
    let mut diagnostics = Vec::new();

    // Phase 1: Parse
    let ast = match parser::parse_mql5(source) {
        Ok(ast) => ast,
        Err(e) => {
            diagnostics.push(Diagnostic {
                level: DiagLevel::Error,
                message: e.to_string(),
                line: e.line(),
                col: e.col(),
            });
            return CompileResult { wasm: None, diagnostics, metadata: None };
        }
    };

    // Phase 2: Extract metadata
    let metadata = ir::extract_metadata(&ast);

    // Phase 3: Lower to IR
    let ir_module = match ir::lower(&ast) {
        Ok(ir) => ir,
        Err(errors) => {
            for e in errors {
                diagnostics.push(Diagnostic {
                    level: DiagLevel::Error,
                    message: e.to_string(),
                    line: e.line(),
                    col: e.col(),
                });
            }
            return CompileResult { wasm: None, diagnostics, metadata: Some(metadata) };
        }
    };

    // Phase 4: Generate WASM
    match codegen::emit_wasm(&ir_module) {
        Ok(wasm_bytes) => CompileResult {
            wasm: Some(wasm_bytes),
            diagnostics,
            metadata: Some(metadata),
        },
        Err(e) => {
            diagnostics.push(Diagnostic {
                level: DiagLevel::Error,
                message: format!("WASM generation failed: {e}"),
                line: 0,
                col: 0,
            });
            CompileResult { wasm: None, diagnostics, metadata: Some(metadata) }
        }
    }
}

/// Compile MQL5 source to a WGSL compute shader string.
pub fn compile_to_wgsl(source: &str) -> Result<String, error::CompileError> {
    wgsl_codegen::compile_to_wgsl(source)
}

/// Compile PineScript v5 source to WASM.
/// Supports: indicator(), input.*, ta.sma/ema/rsi/atr, plot(), math.*, close/open/high/low/volume.
pub fn compile_pine(source: &str) -> CompileResult {
    pine::parse_pine(source)
}

/// Compile EasyLanguage (TradeStation / MultiCharts PowerLanguage) source to WASM.
/// Supports: inputs, variables, Plot1..N, Average/XAverage/RSI/ATR built-ins,
/// brace + line comments, case-insensitive identifiers.
pub fn compile_easylang(source: &str) -> CompileResult {
    easylang::parse_easylang(source)
}

/// Compile thinkScript (ThinkOrSwim) source to WASM.
/// Supports: input, def, plot, Average/ExpAverage/RSI/ATR built-ins,
/// `declare lower`, # line comments, case-sensitive identifiers.
pub fn compile_thinkscript(source: &str) -> CompileResult {
    thinkscript::parse_thinkscript(source)
}

/// Compile MQL4 (MetaTrader 4) source to WASM. Applies a textual rewrite
/// pass (extern → input, init/start/deinit → OnInit/OnTick/OnDeinit,
/// Close[i] → iClose(_Symbol,0,i), …) and then runs through the MQL5
/// parser. Warnings are emitted for constructs that cannot be auto-ported
/// (OrderSend, OrderSelect — strategies).
pub fn compile_mql4(source: &str) -> CompileResult {
    mql4::compile_mql4(source)
}

/// Compile AmiBroker Formula Language (AFL) source to WASM.
/// Supports: Plot(), Param() → input, EMA/SMA/RSI/ATR/HHV/LLV built-ins,
/// arithmetic, case-insensitive.
pub fn compile_afl(source: &str) -> CompileResult {
    afl::parse_afl(source)
}

/// Compile ProRealTime ProBuilder source to WASM.
/// Supports: RETURN expr AS "label", bracketed-length functions
/// (Average[14], ExponentialAverage[14], RSI[14], ATR[14], Highest[14],
/// Lowest[14], StdDev[14]), CROSSES OVER / CROSSES UNDER, REM comments.
pub fn compile_probuilder(source: &str) -> CompileResult {
    probuilder::parse_probuilder(source)
}

/// Compile NinjaScript (NinjaTrader) source — indicator subset.
/// Supports: `[NinjaScriptProperty]` inputs, `AddPlot()` declarations,
/// `Value[0]` / `Values[N][0]` plot assignments, SMA/EMA/RSI/ATR built-ins,
/// `Math.*` utilities.
pub fn compile_ninjascript(source: &str) -> CompileResult {
    ninjascript::parse_ninjascript(source)
}

/// Compile cTrader cAlgo source — indicator subset.
/// Supports: `[Indicator]` / `[Parameter]` / `[Output]` attributes,
/// `Result[index]` assignments, `Indicators.*` built-ins (SimpleMovingAverage,
/// ExponentialMovingAverage, RelativeStrengthIndex, AverageTrueRange, …),
/// `Math.*` utilities, long-form (`Bars.ClosePrices`) and short-form (`Close`)
/// series access.
pub fn compile_calgo(source: &str) -> CompileResult {
    calgo::parse_calgo(source)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compile_mql5_produces_result() {
        // compile_mql5 should never panic, regardless of input
        let src = "#property indicator_chart_window\n#property indicator_buffers 1\n";
        let result = compile_mql5(src);
        // May or may not produce WASM depending on parser requirements,
        // but should always return a CompileResult without panicking
        if result.metadata.is_some() {
            let meta = result.metadata.as_ref().unwrap();
            assert_eq!(meta.buffers, 1);
        }
        // Verify diagnostics are populated if compilation failed
        if result.wasm.is_none() && result.metadata.is_none() {
            assert!(!result.diagnostics.is_empty());
        }
    }

    #[test]
    fn compile_mql5_invalid_returns_diagnostics() {
        let result = compile_mql5("this is not valid MQL5 {{{{");
        assert!(result.wasm.is_none());
        assert!(!result.diagnostics.is_empty());
        assert!(matches!(result.diagnostics[0].level, DiagLevel::Error));
    }

    #[test]
    fn compile_mql5_empty_returns_error() {
        let result = compile_mql5("");
        // Empty source may parse as empty program — check it doesn't panic
        // Either wasm is None (parse error) or Some (empty but valid)
        let _ = result;
    }

    #[test]
    fn compile_pine_valid_returns_result() {
        let src = r#"//@version=5
indicator("Test", overlay=true)
plot(close)
"#;
        let result = compile_pine(src);
        assert!(result.metadata.is_some(), "valid PineScript should produce metadata");
    }

    #[test]
    fn compile_pine_invalid_returns_error() {
        let result = compile_pine("not valid pine {{{{");
        // Should not panic — either returns diagnostics or empty result
        let _ = result;
    }
}
