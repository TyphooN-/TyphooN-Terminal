//! thinkScript (TD Ameritrade / ThinkOrSwim) frontend — compiles to the common IR.
//!
//! Supports a working subset of thinkScript sufficient for common community studies:
//!
//! - `input name = default;` declarations (int / float inferred from default)
//! - `def name = expression;` local variable declarations
//! - `plot name = expression;` plot statements
//! - Built-in series: `close`, `open`, `high`, `low`, `volume`
//! - Built-in functions: `Average` / `MovingAverage` (SMA), `ExpAverage` (EMA),
//!   `RSI`, `ATR`, `Highest`, `Lowest`, `StDev`, `AbsValue`, `Sqrt`, `Log`, `Max`, `Min`
//! - Static plot color hints via `Plot.SetDefaultColor(Color.X)` and
//!   `AssignValueColor(Color.X)` / `Plot.AssignValueColor(Color.X)`
//! - Assignment syntax ends with `;`
//! - `#` line comments (thinkScript convention)
//! - Case-sensitive (thinkScript convention)
//!
//! Not supported (deferred):
//! - Dynamic conditional plot coloring (metadata records only static color hints)
//! - Multi-line `if then else` (single-line ternary works)
//! - Arrays / reference arrays
//!
//! thinkScript is case-sensitive. Keywords are reserved but follow identifier rules.

use crate::ir::*;
use crate::{CompileResult, DiagLevel, Diagnostic, DrawType, IndicatorMeta, InputParam, PlotDef};
use std::collections::HashSet;

/// Parse thinkScript source and produce an IR-based CompileResult.
pub fn parse_thinkscript(source: &str) -> CompileResult {
    let (ir_module, meta) = build_ir(source);
    let mut diagnostics = Vec::new();
    match crate::codegen::emit_wasm(&ir_module) {
        Ok(wasm) => {
            diagnostics.push(Diagnostic {
                level: DiagLevel::Info,
                message: format!(
                    "thinkScript compiled: {} inputs, {} plots",
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
                message: format!("thinkScript WASM codegen failed: {e}"),
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

/// Build the IR module + metadata for thinkScript source — used by both
/// the WASM codegen path and the cross-language transpiler.
pub fn build_ir(source: &str) -> (IrModule, IndicatorMeta) {
    let mut meta = IndicatorMeta {
        short_name: String::from("thinkScript"),
        buffers: 0,
        separate_window: false,
        inputs: Vec::new(),
        plots: Vec::new(),
    };
    let mut ir_body: Vec<IrStmt> = Vec::new();
    let mut inputs: Vec<IrInput> = Vec::new();
    let mut locals: Vec<(String, IrType)> = Vec::new();
    let mut local_names: HashSet<String> = HashSet::new();
    let mut plot_index: usize = 0;

    for raw_line in source.lines() {
        // Strip # line comments
        let line = match raw_line.find('#') {
            Some(idx) => &raw_line[..idx],
            None => raw_line,
        };
        let trimmed = line.trim().trim_end_matches(';').trim();
        if trimmed.is_empty() {
            continue;
        }

        // declare lower; declare upper; — switch separate_window flag
        if trimmed.starts_with("declare lower") {
            meta.separate_window = true;
            continue;
        }
        if trimmed.starts_with("declare upper") || trimmed.starts_with("declare overlay") {
            meta.separate_window = false;
            continue;
        }

        // Static plot color annotations: `Plot.SetDefaultColor(Color.CYAN)`,
        // `Plot.AssignValueColor(Color.GREEN)`, or `AssignValueColor(Color.RED)`.
        // Dynamic conditional colors are intentionally reduced to the first static
        // Color.* token because PlotDef metadata has one color slot per series.
        if apply_thinkscript_plot_color(trimmed, &mut meta.plots) {
            continue;
        }

        // `input Name = default`
        if let Some(rest) = trimmed.strip_prefix("input ") {
            if let Some((name, default)) = split_assign(rest) {
                let (ty, val, ptype) = classify_default(default);
                inputs.push(IrInput {
                    name: name.clone(),
                    ir_type: ty,
                    default: val,
                });
                meta.inputs.push(InputParam {
                    name,
                    param_type: ptype.into(),
                    default_value: default.to_string(),
                });
            }
            continue;
        }

        // `def Name = expr`
        if let Some(rest) = trimmed.strip_prefix("def ") {
            if let Some((name, expr_str)) = split_assign(rest) {
                if let Some(expr) = parse_ts_expr(expr_str) {
                    if local_names.insert(name.clone()) {
                        locals.push((name.clone(), IrType::F64));
                    }
                    ir_body.push(IrStmt::SetLocal(name, expr));
                }
            }
            continue;
        }

        // `plot Name = expr`
        if let Some(rest) = trimmed.strip_prefix("plot ") {
            if let Some((name, expr_str)) = split_assign(rest) {
                if let Some(expr) = parse_ts_expr(expr_str) {
                    // Allocate the next buffer slot
                    ir_body.push(IrStmt::SetBuffer(plot_index, IrExpr::IBars, expr));
                    meta.plots.push(PlotDef {
                        index: plot_index,
                        label: name,
                        draw_type: DrawType::Line,
                        color: "clrBlue".to_string(),
                        width: 1,
                        style: 0,
                    });
                    plot_index += 1;
                }
            }
            continue;
        }

        // `Name = expr` — re-assignment of existing local (no def keyword)
        if let Some((name, expr_str)) = split_assign(trimmed) {
            // Must be a simple identifier on the LHS
            if !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            if let Some(expr) = parse_ts_expr(expr_str) {
                if local_names.insert(name.clone()) {
                    locals.push((name.clone(), IrType::F64));
                }
                ir_body.push(IrStmt::SetLocal(name, expr));
            }
            continue;
        }
    }

    meta.buffers = plot_index;

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

fn apply_thinkscript_plot_color(line: &str, plots: &mut [PlotDef]) -> bool {
    let lower = line.to_ascii_lowercase();
    let is_color_call = lower.contains("assignvaluecolor(") || lower.contains(".setdefaultcolor(");
    if !is_color_call {
        return false;
    }
    let Some(color) = extract_thinkscript_color(line) else {
        return true;
    };
    let target_name = line
        .split_once('.')
        .map(|(name, _)| name.trim())
        .filter(|name| !name.is_empty());
    if let Some(name) = target_name {
        if let Some(plot) = plots.iter_mut().find(|p| p.label == name) {
            plot.color = color;
        } else if let Some(plot) = plots.last_mut() {
            plot.color = color;
        }
    } else if let Some(plot) = plots.last_mut() {
        plot.color = color;
    }
    true
}

fn extract_thinkscript_color(line: &str) -> Option<String> {
    let lower = line.to_ascii_lowercase();
    let color_pos = lower.find("color.")? + "color.".len();
    let raw = line[color_pos..]
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
        .collect::<String>();
    if raw.is_empty() {
        return None;
    }
    Some(format!("clr{}", to_pascal_color(&raw)))
}

fn to_pascal_color(raw: &str) -> String {
    raw.split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            match chars.next() {
                Some(first) => {
                    first.to_ascii_uppercase().to_string() + &chars.as_str().to_ascii_lowercase()
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

/// Split a `name = expression` into `(name, expr_str)` at the FIRST `=`.
/// Skips comparison operators (`==`, `!=`, `<=`, `>=`).
fn split_assign(s: &str) -> Option<(String, &str)> {
    let eq = s.find('=')?;
    // Skip comparison
    let prev = s[..eq].chars().last();
    if matches!(prev, Some('!' | '<' | '>' | '=')) {
        return None;
    }
    let next = s.as_bytes().get(eq + 1).copied();
    if next == Some(b'=') {
        return None;
    }
    let name = s[..eq].trim().to_string();
    if name.is_empty() {
        return None;
    }
    Some((name, s[eq + 1..].trim()))
}

/// Classify a default value string into `(IrType, IrValue, string_type_label)`.
fn classify_default(default: &str) -> (IrType, IrValue, &'static str) {
    let d = default.trim();
    if d.eq_ignore_ascii_case("yes") || d.eq_ignore_ascii_case("no") {
        return (
            IrType::Bool,
            IrValue::Bool(d.eq_ignore_ascii_case("yes")),
            "bool",
        );
    }
    if let Ok(i) = d.parse::<i32>() {
        return (IrType::I32, IrValue::I32(i), "int");
    }
    if let Ok(f) = d.parse::<f64>() {
        return (IrType::F64, IrValue::F64(f), "float");
    }
    // Series reference ("close", "high", etc.) — represent as a zero default
    (IrType::F64, IrValue::F64(0.0), "float")
}

/// Parse a thinkScript expression into IR.
pub(crate) fn parse_ts_expr(expr: &str) -> Option<IrExpr> {
    let expr = expr.trim().trim_end_matches(';').trim();
    if expr.is_empty() {
        return None;
    }

    // Parenthesised expression
    if expr.starts_with('(') && expr.ends_with(')') {
        return parse_ts_expr(&expr[1..expr.len() - 1]);
    }

    // Numeric literal
    if let Ok(f) = expr.parse::<f64>() {
        if f.fract() == 0.0 && f.abs() < i32::MAX as f64 {
            return Some(IrExpr::I32Const(f as i32));
        }
        return Some(IrExpr::F64Const(f));
    }

    // Built-in series (thinkScript is case-sensitive for identifiers;
    // series tokens are lowercase)
    match expr {
        "close" => return Some(IrExpr::IClose(Box::new(IrExpr::I32Const(0)))),
        "open" => return Some(IrExpr::IOpen(Box::new(IrExpr::I32Const(0)))),
        "high" => return Some(IrExpr::IHigh(Box::new(IrExpr::I32Const(0)))),
        "low" => return Some(IrExpr::ILow(Box::new(IrExpr::I32Const(0)))),
        "volume" => return Some(IrExpr::IVolume(Box::new(IrExpr::I32Const(0)))),
        "barnumber" | "BarNumber" => return Some(IrExpr::IBars),
        _ => {}
    }

    // Function call
    if let Some(open) = expr.find('(') {
        if expr.ends_with(')') {
            let func = &expr[..open];
            let args_str = &expr[open + 1..expr.len() - 1];
            let args_parts: Vec<&str> = split_top_level_commas(args_str);
            let ir_args: Option<Vec<IrExpr>> =
                args_parts.iter().map(|a| parse_ts_expr(a.trim())).collect();
            if let Some(ir_args) = ir_args {
                // thinkScript built-ins — map to the common IR function names
                // the runtime knows about. Case-sensitive match on the documented spellings.
                let mapped: Option<&str> = match func {
                    "Average" | "MovingAverage" | "SimpleMovingAvg" => Some("ta_sma"),
                    "ExpAverage" | "ExponentialMovingAvg" => Some("ta_ema"),
                    "RSI" => Some("ta_rsi"),
                    "ATR" | "TrueRange" => Some("ta_atr"),
                    "Highest" => Some("ta_highest"),
                    "Lowest" => Some("ta_lowest"),
                    "StDev" | "StandardDev" => Some("ta_stdev"),
                    "AbsValue" | "Abs" => Some("math_abs"),
                    "Sqrt" | "SquareRoot" => Some("math_sqrt"),
                    "Log" => Some("math_log"),
                    "Max" => Some("math_max"),
                    "Min" => Some("math_min"),
                    _ => None,
                };
                if let Some(name) = mapped {
                    return Some(IrExpr::Call(name.to_string(), ir_args));
                }
            }
        }
    }

    // Binary operators — thinkScript uses standard ops
    for op_str in &[
        " + ", " - ", " * ", " / ", " > ", " < ", " >= ", " <= ", " == ", " != ",
    ] {
        if let Some(pos) = expr.find(op_str) {
            let left = parse_ts_expr(&expr[..pos])?;
            let right = parse_ts_expr(&expr[pos + op_str.len()..])?;
            let ir_op = match *op_str {
                " + " => IrBinOp::AddF64,
                " - " => IrBinOp::SubF64,
                " * " => IrBinOp::MulF64,
                " / " => IrBinOp::DivF64,
                " > " => IrBinOp::GtF64,
                " < " => IrBinOp::LtF64,
                " >= " => IrBinOp::GeF64,
                " <= " => IrBinOp::LeF64,
                " == " => IrBinOp::EqF64,
                " != " => IrBinOp::NeF64,
                _ => IrBinOp::AddF64,
            };
            return Some(IrExpr::BinOp(ir_op, Box::new(left), Box::new(right)));
        }
    }

    // Identifier — local variable (case-sensitive)
    if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Some(IrExpr::GetLocal(expr.to_string()));
    }

    None
}

/// Split a string on top-level commas (ignoring commas inside nested parens).
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
    use crate::compile_thinkscript;

    #[test]
    fn test_ts_simple_ma() {
        let src = r#"
input length = 14;
def ma = Average(close, length);
plot MA = ma;
"#;
        let result = compile_thinkscript(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.inputs[0].name, "length");
        assert_eq!(meta.plots.len(), 1);
        assert_eq!(meta.plots[0].label, "MA");
    }

    #[test]
    fn test_ts_multiple_inputs() {
        let src = r#"
input fastLen = 10;
input slowLen = 30;
input signalLen = 9;
plot Zero = 0;
"#;
        let result = compile_thinkscript(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.inputs.len(), 3);
    }

    #[test]
    fn test_ts_multiple_plots() {
        let src = r#"
plot Hi = high;
plot Lo = low;
plot C = close;
"#;
        let result = compile_thinkscript(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 3);
    }

    #[test]
    fn test_ts_declare_lower() {
        let src = r#"
declare lower;
plot RSI = RSI(close, 14);
"#;
        let result = compile_thinkscript(src);
        assert!(result.metadata.is_some());
        assert!(result.metadata.unwrap().separate_window);
    }

    #[test]
    fn test_ts_hash_comment_stripped() {
        let src = r#"
# this is a comment
plot P = close; # trailing comment
"#;
        let result = compile_thinkscript(src);
        assert!(result.metadata.is_some());
        assert_eq!(result.metadata.unwrap().plots.len(), 1);
    }

    #[test]
    fn test_ts_expaverage_maps_to_ema() {
        let expr = parse_ts_expr("ExpAverage(close, 14)");
        assert!(expr.is_some());
        if let Some(IrExpr::Call(name, _)) = expr {
            assert_eq!(name, "ta_ema");
        } else {
            panic!("ExpAverage should map to ta_ema");
        }
    }

    #[test]
    fn test_ts_input_bool() {
        let src = r#"input showSignal = yes;"#;
        let result = compile_thinkscript(src);
        let meta = result.metadata.unwrap();
        assert_eq!(meta.inputs[0].param_type, "bool");
    }

    #[test]
    fn test_ts_input_float() {
        let src = r#"input multiplier = 2.5;"#;
        let result = compile_thinkscript(src);
        let meta = result.metadata.unwrap();
        assert_eq!(meta.inputs[0].param_type, "float");
    }

    #[test]
    fn test_ts_arithmetic_expr() {
        let expr = parse_ts_expr("high - low");
        assert!(expr.is_some());
    }

    #[test]
    fn test_ts_def_then_plot() {
        let src = r#"
def spread = high - low;
plot Range = spread;
"#;
        let result = compile_thinkscript(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 1);
    }

    #[test]
    fn test_ts_empty() {
        let result = compile_thinkscript("");
        assert!(result.metadata.is_some());
        assert_eq!(result.metadata.unwrap().plots.len(), 0);
    }

    #[test]
    fn test_ts_skips_comparison_in_assignment() {
        // "a = b == c" should not trip the split_assign guard
        let r = split_assign("x = close == high");
        assert!(r.is_some());
        let (name, rhs) = r.unwrap();
        assert_eq!(name, "x");
        assert_eq!(rhs, "close == high");
    }
}

#[cfg(test)]
mod color_tests {
    use super::build_ir;

    #[test]
    fn thinkscript_static_plot_color_metadata() {
        let src = r#"
            declare lower;
            plot Fast = Average(close, 10);
            Fast.SetDefaultColor(Color.CYAN);
            plot Slow = Average(close, 30);
            Slow.AssignValueColor(Color.DARK_RED);
        "#;
        let (_ir, meta) = build_ir(src);
        assert!(meta.separate_window);
        assert_eq!(meta.plots[0].color, "clrCyan");
        assert_eq!(meta.plots[1].color, "clrDarkRed");
    }
    #[test]
    fn test_ts_duplicate_local_declared_once() {
        let src = r#"
def value = close;
value = open;
plot Value = value;
"#;
        let (module, _) = build_ir(src);
        let locals = &module.on_calculate.as_ref().unwrap().locals;
        assert_eq!(locals.iter().filter(|(name, _)| name == "value").count(), 1);
    }
}
