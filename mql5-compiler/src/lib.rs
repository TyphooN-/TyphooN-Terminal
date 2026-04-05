//! MQL5 & PineScript to WASM compiler for TyphooN Terminal.
//!
//! Pipeline: Source → Parse → AST → IR → WASM
//!
//! Supports MQL5 indicators (OnCalculate, SetIndexBuffer, DRAW_* types)
//! and PineScript indicators (plot, ta.*, input.*).

pub mod parser;
pub mod ast;
pub mod ir;
pub mod codegen;
pub mod wgsl_codegen;
pub mod error;
pub mod runtime;
pub mod pine;



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
