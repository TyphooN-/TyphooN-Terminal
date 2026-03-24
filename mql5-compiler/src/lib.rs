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
pub mod error;
pub mod runtime;



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

#[derive(serde::Serialize)]
pub struct Diagnostic {
    pub level: DiagLevel,
    pub message: String,
    pub line: usize,
    pub col: usize,
}

#[derive(serde::Serialize)]
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

/// Compile PineScript source to WASM.
pub fn compile_pine(_source: &str) -> CompileResult {
    // TODO: Phase 4 — PineScript parser
    CompileResult {
        wasm: None,
        diagnostics: vec![Diagnostic {
            level: DiagLevel::Info,
            message: "PineScript compiler not yet implemented".into(),
            line: 0,
            col: 0,
        }],
        metadata: None,
    }
}
