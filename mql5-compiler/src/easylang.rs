//! EasyLanguage (TradeStation) frontend — compiles to the common IR.
//!
//! Supports a working subset of EasyLanguage / MultiCharts PowerLanguage
//! sufficient for the ~90% of published community indicators:
//!
//! - `inputs:` block (with or without typed declarations)
//! - `variables:` / `vars:` block
//! - Built-in series: `Close`, `Open`, `High`, `Low`, `Volume`
//! - Built-in functions: `Average`, `XAverage` (EMA), `RSI`, `ATR`,
//!   `Highest`, `Lowest`, `StdDev`, `AbsValue`, `SquareRoot`, `Log`
//! - Assignment statements: `Name = expression;`
//! - `Plot1..PlotN(value, "label")`
//! - Case-insensitive keywords (EL convention)
//! - `//` and `{ }` comments
//!
//! Not supported (deferred):
//! - `if/then/else` with multi-line blocks
//! - `Buy`/`Sell`/`SellShort`/`BuyToCover` signals (no trade sim path)
//! - User-defined functions
//! - Arrays
//!
//! EL is case-insensitive; we lowercase identifiers for lookup.

use crate::ir::*;
use crate::{CompileResult, DiagLevel, Diagnostic, DrawType, IndicatorMeta, InputParam, PlotDef};

/// Parse EasyLanguage source and produce an IR-based CompileResult.
pub fn parse_easylang(source: &str) -> CompileResult {
    let (ir_module, meta) = build_ir(source);
    let mut diagnostics = Vec::new();
    match crate::codegen::emit_wasm(&ir_module) {
        Ok(wasm) => {
            diagnostics.push(Diagnostic {
                level: DiagLevel::Info,
                message: format!(
                    "EasyLanguage compiled: {} inputs, {} plots",
                    meta.inputs.len(),
                    meta.plots.len()
                ),
                line: 0,
                col: 0,
            });
            CompileResult {
                wasm: Some(wasm),
                diagnostics,
                metadata: Some(meta),
            }
        }
        Err(e) => {
            diagnostics.push(Diagnostic {
                level: DiagLevel::Error,
                message: format!("EasyLanguage WASM codegen failed: {e}"),
                line: 0,
                col: 0,
            });
            CompileResult {
                wasm: None,
                diagnostics,
                metadata: Some(meta),
            }
        }
    }
}

/// Build the IR module + metadata for EasyLanguage source — used by both
/// the WASM codegen path and the cross-language transpiler.
pub fn build_ir(source: &str) -> (IrModule, IndicatorMeta) {
    let mut meta = IndicatorMeta {
        short_name: String::from("EasyLanguage"),
        buffers: 0,
        separate_window: false,
        inputs: Vec::new(),
        plots: Vec::new(),
    };
    let mut ir_body: Vec<IrStmt> = Vec::new();
    let mut inputs: Vec<IrInput> = Vec::new();
    let mut locals: Vec<(String, IrType)> = Vec::new();

    // Strip {} comments, then line-by-line scan with // comment removal.
    let cleaned = strip_brace_comments(source);

    let mut in_inputs = false;
    let mut in_vars = false;

    for raw_line in cleaned.lines() {
        // Strip // comments
        let line = match raw_line.find("//") {
            Some(idx) => &raw_line[..idx],
            None => raw_line,
        };
        let trimmed = line.trim().trim_end_matches(';');
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();

        // Section openers: "inputs:" / "input:" / "variables:" / "vars:"
        if lower.starts_with("inputs:") || lower.starts_with("input:") {
            in_inputs = true;
            in_vars = false;
            let after_colon = &trimmed[trimmed.find(':').unwrap_or(0) + 1..];
            parse_el_input_list(after_colon, &mut inputs, &mut meta.inputs);
            continue;
        }
        if lower.starts_with("variables:") || lower.starts_with("vars:") {
            in_inputs = false;
            in_vars = true;
            let after_colon = &trimmed[trimmed.find(':').unwrap_or(0) + 1..];
            parse_el_var_list(after_colon, &mut locals);
            continue;
        }

        // Close section headers as soon as we see anything that looks like a
        // plot statement or assignment — those can never be continuation lines.
        let looks_like_statement = lower.starts_with("plot") || lower.contains('=');
        if looks_like_statement {
            in_inputs = false;
            in_vars = false;
        }

        // Section continuation — comma-separated items across lines
        if in_inputs {
            parse_el_input_list(trimmed, &mut inputs, &mut meta.inputs);
            continue;
        }
        if in_vars {
            parse_el_var_list(trimmed, &mut locals);
            continue;
        }

        // Plot statement: Plot1(value) / Plot1(value, "label")
        if let Some(rest) = el_plot_prefix(&lower) {
            // rest starts at the matching Plot<N>( content. The slice is
            // taken from the lowercased line, but since EL is ASCII, the
            // byte offsets match the original `trimmed` — use `trimmed` for
            // the label so we preserve the user's original capitalisation.
            let plot_idx = rest.0;
            let args_lower = rest.1;
            let offset = trimmed.len() - args_lower.len();
            let args_orig = &trimmed[offset..];
            let args = extract_parens(args_orig);
            let parts: Vec<&str> = split_top_level_commas(args);
            if let Some(first) = parts.first() {
                if let Some(expr) = parse_el_expr(first.trim()) {
                    // Always pass current-bar (index 0) semantics via IBars for runtime time axis
                    ir_body.push(IrStmt::SetBuffer(plot_idx, IrExpr::IBars, expr));
                    let label = parts
                        .get(1)
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .unwrap_or_else(|| format!("Plot{}", plot_idx + 1));
                    if meta.plots.iter().all(|p| p.index != plot_idx) {
                        meta.plots.push(PlotDef {
                            index: plot_idx,
                            label,
                            draw_type: DrawType::Line,
                            color: "clrBlue".to_string(),
                            width: 1,
                            style: 0,
                        });
                    }
                }
            }
            continue;
        }

        // Assignment statement: name = expression
        if let Some(eq_pos) = trimmed.find('=') {
            // Skip comparison operators
            let prev = trimmed[..eq_pos].chars().last();
            if matches!(prev, Some('!' | '<' | '>' | '=')) {
                continue;
            }
            let lhs = trimmed[..eq_pos].trim();
            let rhs = trimmed[eq_pos + 1..].trim();
            if lhs.is_empty() || !lhs.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            let lhs_lower = lhs.to_ascii_lowercase();
            if let Some(expr) = parse_el_expr(rhs) {
                // Auto-declare if not already a local
                if !locals
                    .iter()
                    .any(|(n, _)| n.eq_ignore_ascii_case(&lhs_lower))
                {
                    locals.push((lhs_lower.clone(), IrType::F64));
                }
                ir_body.push(IrStmt::SetLocal(lhs_lower, expr));
            }
            continue;
        }
    }

    meta.buffers = meta.plots.len();

    let on_calculate = IrFunction {
        name: "OnCalculate".into(),
        params: vec![
            ("rates_total".into(), IrType::I32),
            ("prev_calculated".into(), IrType::I32),
        ],
        return_type: IrType::I32,
        body: ir_body,
        locals,
    };

    let ir_module = IrModule {
        buffers: Vec::new(),
        inputs,
        functions: Vec::new(),
        on_calculate: Some(on_calculate),
        on_init: None,
        globals: Vec::new(),
    };
    (ir_module, meta)
}

/// If the (lowercased) line starts with "plot<N>(", return (index, original-case prefix+rest).
fn el_plot_prefix(lower: &str) -> Option<(usize, &str)> {
    if !lower.starts_with("plot") {
        return None;
    }
    let after = &lower[4..];
    let num_end = after.find('(')?;
    let num: usize = after[..num_end].parse().ok()?;
    if num == 0 {
        return None;
    }
    // Return (0-based index, the "plotN(...)" slice itself)
    Some((num - 1, lower.get(4 + num_end..)?))
}

/// Parse a comma-separated list of `Name(default)` pairs as typed inputs.
/// EL syntax: `Length(14), Source(Close), Multiplier(2.0)`
fn parse_el_input_list(list: &str, inputs: &mut Vec<IrInput>, meta_inputs: &mut Vec<InputParam>) {
    let list = list.trim().trim_end_matches(';').trim_end_matches(',');
    if list.is_empty() {
        return;
    }
    for item in split_top_level_commas(list) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        // item: "Length(14)" or "Source(Close)"
        if let Some(open) = item.find('(') {
            let name = item[..open].trim().to_string();
            let default_str = item[open + 1..].trim_end_matches(')').trim();
            if name.is_empty() {
                continue;
            }
            // Try int first, then float, else string
            let (ty, val) = if let Ok(i) = default_str.parse::<i32>() {
                (IrType::I32, IrValue::I32(i))
            } else if let Ok(f) = default_str.parse::<f64>() {
                (IrType::F64, IrValue::F64(f))
            } else {
                // Series reference ("Close", "High", etc.) — default to 0.0
                (IrType::F64, IrValue::F64(0.0))
            };
            let param_type = match ty {
                IrType::I32 => "int",
                IrType::F64 => "float",
                _ => "string",
            };
            inputs.push(IrInput {
                name: name.to_ascii_lowercase(),
                ir_type: ty,
                default: val,
            });
            meta_inputs.push(InputParam {
                name,
                param_type: param_type.to_string(),
                default_value: default_str.to_string(),
            });
        }
    }
}

/// Parse a comma-separated list of `Name(initial)` pairs as local variables.
fn parse_el_var_list(list: &str, locals: &mut Vec<(String, IrType)>) {
    let list = list.trim().trim_end_matches(';').trim_end_matches(',');
    if list.is_empty() {
        return;
    }
    for item in split_top_level_commas(list) {
        let item = item.trim();
        if item.is_empty() {
            continue;
        }
        let name = if let Some(open) = item.find('(') {
            item[..open].trim().to_string()
        } else {
            item.to_string()
        };
        if !name.is_empty() && !locals.iter().any(|(n, _)| n.eq_ignore_ascii_case(&name)) {
            locals.push((name.to_ascii_lowercase(), IrType::F64));
        }
    }
}

/// Parse an EL expression into IR.
pub(crate) fn parse_el_expr(expr: &str) -> Option<IrExpr> {
    let expr = expr.trim().trim_end_matches(';').trim();
    if expr.is_empty() {
        return None;
    }

    // Parenthesised expression
    if expr.starts_with('(') && expr.ends_with(')') {
        return parse_el_expr(&expr[1..expr.len() - 1]);
    }

    // Numeric literal
    if let Ok(f) = expr.parse::<f64>() {
        if f.fract() == 0.0 && f.abs() < i32::MAX as f64 {
            return Some(IrExpr::I32Const(f as i32));
        }
        return Some(IrExpr::F64Const(f));
    }

    let lower = expr.to_ascii_lowercase();

    // Built-in series (case-insensitive)
    match lower.as_str() {
        "close" | "c" => return Some(IrExpr::IClose(Box::new(IrExpr::I32Const(0)))),
        "open" | "o" => return Some(IrExpr::IOpen(Box::new(IrExpr::I32Const(0)))),
        "high" | "h" => return Some(IrExpr::IHigh(Box::new(IrExpr::I32Const(0)))),
        "low" | "l" => return Some(IrExpr::ILow(Box::new(IrExpr::I32Const(0)))),
        "volume" | "v" => return Some(IrExpr::IVolume(Box::new(IrExpr::I32Const(0)))),
        "currentbar" | "barnumber" => return Some(IrExpr::IBars),
        _ => {}
    }

    // Built-in functions — dispatch by lowercase prefix
    // EL function calls: Name(arg1, arg2, ...)
    if let Some(open) = lower.find('(') {
        if expr.ends_with(')') {
            let func = &lower[..open];
            let args_str = &expr[open + 1..expr.len() - 1];
            let args_parts: Vec<&str> = split_top_level_commas(args_str);
            let ir_args: Option<Vec<IrExpr>> =
                args_parts.iter().map(|a| parse_el_expr(a.trim())).collect();
            if let Some(ir_args) = ir_args {
                // Map EL func names to IR calls the runtime knows about.
                let mapped: Option<&str> = match func {
                    "average" | "avg" | "sma" => Some("ta_sma"),
                    "xaverage" | "ema" => Some("ta_ema"),
                    "rsi" => Some("ta_rsi"),
                    "atr" => Some("ta_atr"),
                    "highest" => Some("ta_highest"),
                    "lowest" => Some("ta_lowest"),
                    "stddev" | "standarddev" => Some("ta_stdev"),
                    "absvalue" | "absolute" | "abs" => Some("math_abs"),
                    "squareroot" | "sqrt" => Some("math_sqrt"),
                    "log" => Some("math_log"),
                    "maxlist" | "maximum" | "max" => Some("math_max"),
                    "minlist" | "minimum" | "min" => Some("math_min"),
                    _ => None,
                };
                if let Some(name) = mapped {
                    return Some(IrExpr::Call(name.to_string(), ir_args));
                }
            }
        }
    }

    // Binary operators (left-to-right, no precedence — same as pine.rs)
    for op_str in &[
        " + ", " - ", " * ", " / ", " > ", " < ", " >= ", " <= ", " = ", " <> ",
    ] {
        if let Some(pos) = expr.find(op_str) {
            let left = parse_el_expr(&expr[..pos])?;
            let right = parse_el_expr(&expr[pos + op_str.len()..])?;
            let ir_op = match *op_str {
                " + " => IrBinOp::AddF64,
                " - " => IrBinOp::SubF64,
                " * " => IrBinOp::MulF64,
                " / " => IrBinOp::DivF64,
                " > " => IrBinOp::GtF64,
                " < " => IrBinOp::LtF64,
                " >= " => IrBinOp::GeF64,
                " <= " => IrBinOp::LeF64,
                " = " => IrBinOp::EqF64,
                " <> " => IrBinOp::NeF64,
                _ => IrBinOp::AddF64,
            };
            return Some(IrExpr::BinOp(ir_op, Box::new(left), Box::new(right)));
        }
    }

    // Identifier — local variable reference (EL is case-insensitive)
    if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Some(IrExpr::GetLocal(lower));
    }

    None
}

/// Strip `{ ... }` block comments. EL treats braces as multi-line comments.
fn strip_brace_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut depth = 0usize;
    for ch in src.chars() {
        match ch {
            '{' => depth += 1,
            '}' => {
                if depth > 0 {
                    depth -= 1
                }
            }
            _ => {
                if depth == 0 {
                    out.push(ch);
                }
            }
        }
    }
    out
}

/// Extract inside of outermost parens (first `(` to matching `)`).
fn extract_parens(s: &str) -> &str {
    let start = s.find('(').map(|i| i + 1).unwrap_or(0);
    let end = s.rfind(')').unwrap_or(s.len());
    &s[start..end]
}

/// Split a string on top-level commas (ignore commas inside nested parens).
fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            ',' if depth == 0 => {
                parts.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    if start < s.len() {
        parts.push(&s[start..]);
    }
    parts
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_easylang;

    #[test]
    fn test_el_simple_plot() {
        let src = r#"
inputs: Length(14);
variables: MA(0);

MA = Average(Close, Length);
Plot1(MA, "Moving Average");
"#;
        let result = compile_easylang(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 1);
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.inputs[0].name, "Length");
    }

    #[test]
    fn test_el_multi_input() {
        let src = r#"
inputs: Fast(10), Slow(30), Signal(9);
Plot1(Close, "Price");
"#;
        let result = compile_easylang(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.inputs.len(), 3);
        assert_eq!(meta.inputs[0].name, "Fast");
        assert_eq!(meta.inputs[2].name, "Signal");
    }

    #[test]
    fn test_el_multiple_plots() {
        let src = r#"
Plot1(Close, "C");
Plot2(High, "H");
Plot3(Low, "L");
"#;
        let result = compile_easylang(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 3);
    }

    #[test]
    fn test_el_brace_comment_stripped() {
        let src = r#"
{ This is a multi-line
  comment that should be ignored }
Plot1(Close, "X");
"#;
        let result = compile_easylang(src);
        assert!(result.metadata.is_some());
        assert_eq!(result.metadata.unwrap().plots.len(), 1);
    }

    #[test]
    fn test_el_line_comment_stripped() {
        let src = r#"
Plot1(Close); // this is a trailing comment
"#;
        let result = compile_easylang(src);
        assert!(result.metadata.is_some());
        assert_eq!(result.metadata.unwrap().plots.len(), 1);
    }

    #[test]
    fn test_el_case_insensitive() {
        let src = r#"
INPUTS: LENGTH(14);
MA = AVERAGE(CLOSE, LENGTH);
plot1(MA);
"#;
        let result = compile_easylang(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.plots.len(), 1);
    }

    #[test]
    fn test_el_binary_op() {
        // Ensure parse_el_expr handles arithmetic
        let result = parse_el_expr("Close + 1.5");
        assert!(result.is_some());
    }

    #[test]
    fn test_el_empty_source() {
        let result = compile_easylang("");
        // Empty source shouldn't panic; no plots or inputs
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 0);
    }

    #[test]
    fn test_el_builtin_series_shortcuts() {
        assert!(parse_el_expr("C").is_some());
        assert!(parse_el_expr("H").is_some());
        assert!(parse_el_expr("L").is_some());
        assert!(parse_el_expr("O").is_some());
        assert!(parse_el_expr("V").is_some());
    }

    #[test]
    fn test_el_xaverage_maps_to_ema() {
        let expr = parse_el_expr("XAverage(Close, 14)");
        assert!(expr.is_some());
        if let Some(IrExpr::Call(name, _)) = expr {
            assert_eq!(name, "ta_ema");
        } else {
            panic!("XAverage should map to ta_ema");
        }
    }

    #[test]
    fn test_el_split_top_level_commas() {
        let parts = split_top_level_commas("a, b, c");
        assert_eq!(parts.len(), 3);
        // Nested parens stay grouped
        let nested = split_top_level_commas("Average(Close, 14), 2");
        assert_eq!(nested.len(), 2);
    }
}
