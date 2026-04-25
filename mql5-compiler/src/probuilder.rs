//! ProRealTime ProBuilder / ProScreener frontend.
//!
//! ProRealTime is the dominant European retail charting platform. ProBuilder
//! is their BASIC-like indicator language. Simple line-based syntax.
//!
//! Supported:
//! - `REM ...` and `//` comments (both styles in practice)
//! - Variable assignment: `name = expression`
//! - Built-in series: `Close`, `Open`, `High`, `Low`, `Volume` (lowercased too)
//! - Built-in functions with square-bracket length notation:
//!   `Average[length](source)` / `ExponentialAverage[length](source)`
//!   `RSI[length](source)` / `ATR[length]` / `Highest[length](source)`
//!   `Lowest[length](source)` / `StdDev[length](source)`
//! - `RETURN expr` / `RETURN expr AS "label"` (multi-return supported)
//! - `CROSSES OVER` / `CROSSES UNDER` operators
//! - Arithmetic + comparison
//!
//! Deferred:
//! - `IF ... THEN ... ELSE ... ENDIF` multi-line blocks
//! - `FOR ... NEXT` loops
//! - User-defined functions

use crate::ir::*;
use crate::{CompileResult, DiagLevel, Diagnostic, DrawType, IndicatorMeta, PlotDef};

pub fn parse_probuilder(source: &str) -> CompileResult {
    let (ir_module, meta) = build_ir(source);
    let mut diagnostics = Vec::new();
    match crate::codegen::emit_wasm(&ir_module) {
        Ok(wasm) => {
            diagnostics.push(Diagnostic {
                level: DiagLevel::Info,
                message: format!("ProBuilder compiled: {} returns", meta.plots.len()),
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
                message: format!("ProBuilder codegen failed: {e}"),
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

pub fn build_ir(source: &str) -> (IrModule, IndicatorMeta) {
    let mut meta = IndicatorMeta {
        short_name: String::from("ProBuilder"),
        buffers: 0,
        separate_window: false,
        inputs: Vec::new(),
        plots: Vec::new(),
    };
    let mut ir_body: Vec<IrStmt> = Vec::new();
    let inputs: Vec<IrInput> = Vec::new();
    let mut locals: Vec<(String, IrType)> = Vec::new();
    let mut return_count = 0usize;

    for raw_line in source.lines() {
        // Handle both // and REM comments (ProBuilder uses REM traditionally)
        let line = if let Some(idx) = raw_line.find("//") {
            &raw_line[..idx]
        } else {
            raw_line
        };
        let trimmed_raw = line.trim();
        // REM-style comment: whole line
        let upper_first: String = trimmed_raw
            .chars()
            .take(4)
            .collect::<String>()
            .to_ascii_uppercase();
        if upper_first.starts_with("REM ") || trimmed_raw.eq_ignore_ascii_case("REM") {
            continue;
        }
        let trimmed = trimmed_raw;
        if trimmed.is_empty() {
            continue;
        }
        let upper = trimmed.to_ascii_uppercase();

        // RETURN statement(s): `RETURN expr1 [AS "label1"], expr2 [AS "label2"]`
        if upper.starts_with("RETURN ") {
            let rest = &trimmed[7..];
            // Split on top-level commas to support multi-return
            for segment in split_top_level_commas(rest) {
                let seg = segment.trim();
                if seg.is_empty() {
                    continue;
                }
                // Look for ` AS "label"` suffix (case-insensitive)
                let (expr_part, label) = if let Some(pos) = find_case_insensitive(seg, " AS ") {
                    let label_part = seg[pos + 4..].trim().trim_matches('"').to_string();
                    (seg[..pos].trim(), label_part)
                } else {
                    (seg, format!("Plot{}", return_count + 1))
                };
                if let Some(expr) = parse_pb_expr(expr_part) {
                    ir_body.push(IrStmt::SetBuffer(return_count, IrExpr::IBars, expr));
                    meta.plots.push(PlotDef {
                        index: return_count,
                        label,
                        draw_type: DrawType::Line,
                        color: "clrBlue".into(),
                        width: 1,
                        style: 0,
                    });
                    return_count += 1;
                }
            }
            continue;
        }

        // Assignment: name = expr
        if let Some(eq_pos) = trimmed.find('=') {
            let prev = trimmed[..eq_pos].chars().last();
            if matches!(prev, Some('!' | '<' | '>' | '=')) {
                continue;
            }
            let lhs = trimmed[..eq_pos].trim();
            let rhs = trimmed[eq_pos + 1..].trim();
            if lhs.is_empty() || !lhs.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }
            if let Some(expr) = parse_pb_expr(rhs) {
                let name = lhs.to_ascii_lowercase();
                if !locals.iter().any(|(n, _)| n == &name) {
                    locals.push((name.clone(), IrType::F64));
                }
                ir_body.push(IrStmt::SetLocal(name, expr));
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
    let _ = return_count;
    (ir_module, meta)
}

fn parse_pb_expr(expr: &str) -> Option<IrExpr> {
    let expr = expr.trim();
    if expr.is_empty() {
        return None;
    }
    if expr.starts_with('(') && expr.ends_with(')') {
        return parse_pb_expr(&expr[1..expr.len() - 1]);
    }
    if let Ok(f) = expr.parse::<f64>() {
        if f.fract() == 0.0 && f.abs() < i32::MAX as f64 {
            return Some(IrExpr::I32Const(f as i32));
        }
        return Some(IrExpr::F64Const(f));
    }

    let lower = expr.to_ascii_lowercase();
    match lower.as_str() {
        "close" => return Some(IrExpr::IClose(Box::new(IrExpr::I32Const(0)))),
        "open" => return Some(IrExpr::IOpen(Box::new(IrExpr::I32Const(0)))),
        "high" => return Some(IrExpr::IHigh(Box::new(IrExpr::I32Const(0)))),
        "low" => return Some(IrExpr::ILow(Box::new(IrExpr::I32Const(0)))),
        "volume" => return Some(IrExpr::IVolume(Box::new(IrExpr::I32Const(0)))),
        "barindex" => return Some(IrExpr::IBars),
        _ => {}
    }

    // CROSSES OVER / CROSSES UNDER binary operator (case-insensitive)
    if let Some(pos) = find_case_insensitive(expr, " CROSSES OVER ") {
        let left = parse_pb_expr(expr[..pos].trim())?;
        let right = parse_pb_expr(expr[pos + 14..].trim())?;
        return Some(IrExpr::Call("ta_crossover".into(), vec![left, right]));
    }
    if let Some(pos) = find_case_insensitive(expr, " CROSSES UNDER ") {
        let left = parse_pb_expr(expr[..pos].trim())?;
        let right = parse_pb_expr(expr[pos + 15..].trim())?;
        return Some(IrExpr::Call("ta_crossunder".into(), vec![left, right]));
    }

    // Bracketed-length function form: `Func[length](source)`
    if let Some(lb) = expr.find('[') {
        let name = expr[..lb].trim().to_ascii_lowercase();
        if let Some(rb) = expr[lb..].find(']') {
            let length_str = &expr[lb + 1..lb + rb];
            // After `]` either end-of-expr or `(source)`
            let rest = expr[lb + rb + 1..].trim();
            let length_expr = parse_pb_expr(length_str.trim())?;
            let source_expr = if rest.starts_with('(') && rest.ends_with(')') {
                parse_pb_expr(&rest[1..rest.len() - 1])?
            } else if rest.is_empty() {
                // ATR[14] with no source — use close
                IrExpr::IClose(Box::new(IrExpr::I32Const(0)))
            } else {
                return None;
            };
            let mapped: Option<&str> = match name.as_str() {
                "average" | "ma" | "sma" => Some("ta_sma"),
                "exponentialaverage" | "ema" => Some("ta_ema"),
                "rsi" => Some("ta_rsi"),
                "atr" => Some("ta_atr"),
                "highest" => Some("ta_highest"),
                "lowest" => Some("ta_lowest"),
                "std" | "stddev" => Some("ta_stdev"),
                _ => None,
            };
            if let Some(name) = mapped {
                // ATR takes length only — special-case
                if name == "ta_atr" {
                    return Some(IrExpr::Call(name.into(), vec![length_expr]));
                }
                return Some(IrExpr::Call(name.into(), vec![source_expr, length_expr]));
            }
        }
    }

    // Plain function call: name(args)
    if let Some(open) = lower.find('(') {
        if expr.ends_with(')') {
            let func = &lower[..open];
            let args_str = &expr[open + 1..expr.len() - 1];
            let parts = split_top_level_commas(args_str);
            let ir_args: Option<Vec<IrExpr>> =
                parts.iter().map(|a| parse_pb_expr(a.trim())).collect();
            if let Some(ir_args) = ir_args {
                let mapped: Option<&str> = match func {
                    "abs" => Some("math_abs"),
                    "sqrt" => Some("math_sqrt"),
                    "log" => Some("math_log"),
                    "max" => Some("math_max"),
                    "min" => Some("math_min"),
                    _ => None,
                };
                if let Some(name) = mapped {
                    return Some(IrExpr::Call(name.into(), ir_args));
                }
            }
        }
    }

    for op_str in &[
        " + ", " - ", " * ", " / ", " >= ", " <= ", " > ", " < ", " <> ", " = ",
    ] {
        if let Some(pos) = expr.find(op_str) {
            let left = parse_pb_expr(&expr[..pos])?;
            let right = parse_pb_expr(&expr[pos + op_str.len()..])?;
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

    if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Some(IrExpr::GetLocal(lower));
    }
    None
}

fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut sq = 0i32;
    let mut start = 0usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => depth = (depth - 1).max(0),
            '[' => sq += 1,
            ']' => sq = (sq - 1).max(0),
            ',' if depth == 0 && sq == 0 => {
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

fn find_case_insensitive(haystack: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }
    let hl = haystack.to_ascii_lowercase();
    let nl = needle.to_ascii_lowercase();
    hl.find(&nl)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_probuilder;

    #[test]
    fn test_pb_simple_ema_return() {
        let src = r#"
REM Simple EMA cross
ema20 = ExponentialAverage[20](close)
ema50 = ExponentialAverage[50](close)
c1 = ema20 CROSSES OVER ema50
RETURN c1 AS "Cross"
"#;
        let result = compile_probuilder(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 1);
        assert_eq!(meta.plots[0].label, "Cross");
    }

    #[test]
    fn test_pb_multi_return() {
        let src = r#"
fast = Average[10](close)
slow = Average[20](close)
RETURN fast AS "Fast", slow AS "Slow"
"#;
        let result = compile_probuilder(src);
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 2);
        assert_eq!(meta.plots[0].label, "Fast");
        assert_eq!(meta.plots[1].label, "Slow");
    }

    #[test]
    fn test_pb_rem_comment_stripped() {
        let src = r#"
REM this is a comment
RETURN close
"#;
        let result = compile_probuilder(src);
        assert_eq!(result.metadata.unwrap().plots.len(), 1);
    }

    #[test]
    fn test_pb_slash_comment_stripped() {
        let src = r#"
// trailing comment
RETURN close
"#;
        let result = compile_probuilder(src);
        assert_eq!(result.metadata.unwrap().plots.len(), 1);
    }

    #[test]
    fn test_pb_arithmetic() {
        let src = r#"
mid = (high + low) / 2
RETURN mid
"#;
        let result = compile_probuilder(src);
        assert_eq!(result.metadata.unwrap().plots.len(), 1);
    }

    #[test]
    fn test_pb_case_insensitive() {
        let src = r#"
EMA20 = EXPONENTIALAVERAGE[20](CLOSE)
return EMA20 as "ema"
"#;
        let result = compile_probuilder(src);
        assert_eq!(result.metadata.unwrap().plots.len(), 1);
    }

    #[test]
    fn test_pb_crosses_under() {
        let src = r#"
fast = Average[10](close)
slow = Average[20](close)
x = fast CROSSES UNDER slow
RETURN x
"#;
        let result = compile_probuilder(src);
        assert_eq!(result.metadata.unwrap().plots.len(), 1);
    }

    #[test]
    fn test_pb_atr_no_source() {
        let src = r#"
a = ATR[14]
RETURN a AS "ATR"
"#;
        let result = compile_probuilder(src);
        assert_eq!(result.metadata.unwrap().plots.len(), 1);
    }

    #[test]
    fn test_pb_exp_maps_to_ema() {
        if let Some(IrExpr::Call(name, _)) = parse_pb_expr("ExponentialAverage[14](close)") {
            assert_eq!(name, "ta_ema");
        } else {
            panic!("ExponentialAverage should map to ta_ema");
        }
    }

    #[test]
    fn test_pb_empty_source() {
        let result = compile_probuilder("");
        assert!(result.metadata.is_some());
    }
}
