//! AFL (AmiBroker Formula Language) frontend.
//!
//! AmiBroker has a 20+ year legacy and one of the largest indicator archives
//! on the web. AFL is C-like, case-insensitive, statement-terminated by `;`,
//! and vector-oriented: every variable is an implicit array over bars.
//!
//! Supported:
//! - `//` line comments, `/* ... */` block comments
//! - Variable assignment: `name = expression;`
//! - Built-in price series: `Close`, `Open`, `High`, `Low`, `Volume`,
//!   `C`, `O`, `H`, `L`, `V` shortcuts
//! - Built-in functions mapped to IR calls:
//!   `MA`/`SMA`/`Average` → `ta_sma`
//!   `EMA`/`ExpMovingAvg` → `ta_ema`
//!   `RSI` → `ta_rsi`, `ATR` → `ta_atr`
//!   `HHV`/`Highest` → `ta_highest`, `LLV`/`Lowest` → `ta_lowest`
//!   `StDev`/`StdDev` → `ta_stdev`
//!   `AbsValue`/`abs` → `math_abs`, `sqrt` → `math_sqrt`, `log` → `math_log`
//!   `Max`/`Min` → `math_max`/`math_min`
//! - `Plot(value, "title", color, style);`
//! - `Param("label", default, min, max, step)` — default becomes input
//! - `IIf(cond, a, b)` ternary/select expressions
//!
//! Deferred:
//! - `Buy`/`Sell`/`Short`/`Cover` signals (no trade sim yet)
//! - User-defined functions (`function name() { ... }`)
//! - Matrix/array slicing

use crate::ir::*;
use crate::{CompileResult, DiagLevel, Diagnostic, DrawType, IndicatorMeta, InputParam, PlotDef};

pub fn parse_afl(source: &str) -> CompileResult {
    let (ir_module, meta) = build_ir(source);
    let mut diagnostics = Vec::new();
    match crate::codegen::emit_wasm(&ir_module) {
        Ok(wasm) => {
            diagnostics.push(Diagnostic {
                level: DiagLevel::Info,
                message: format!(
                    "AFL compiled: {} inputs, {} plots",
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
                message: format!("AFL codegen failed: {e}"),
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
        short_name: String::from("AFL"),
        buffers: 0,
        separate_window: false,
        inputs: Vec::new(),
        plots: Vec::new(),
    };
    let mut ir_body: Vec<IrStmt> = Vec::new();
    let mut inputs: Vec<IrInput> = Vec::new();
    let mut locals: Vec<(String, IrType)> = Vec::new();
    let mut plot_count = 0usize;

    let cleaned = strip_block_comments(source);
    for raw_line in cleaned.lines() {
        let line = match raw_line.find("//") {
            Some(idx) => &raw_line[..idx],
            None => raw_line,
        };
        let trimmed = line.trim().trim_end_matches(';');
        if trimmed.is_empty() {
            continue;
        }
        let lower = trimmed.to_ascii_lowercase();

        // _SECTION_BEGIN("Name") — short name
        if lower.starts_with("_section_begin(") {
            if let Some(start) = trimmed.find('"') {
                if let Some(end) = trimmed[start + 1..].find('"') {
                    meta.short_name = trimmed[start + 1..start + 1 + end].to_string();
                }
            }
            continue;
        }
        if lower.starts_with("_section_end") {
            continue;
        }

        // Plot(value, "title", color, style)
        if lower.starts_with("plot(") && trimmed.ends_with(')') {
            let args_str = &trimmed[5..trimmed.len() - 1];
            let parts = split_top_level_commas(args_str);
            if let Some(first) = parts.first() {
                if let Some(expr) = parse_afl_expr(first.trim()) {
                    let label = parts
                        .get(1)
                        .map(|s| s.trim().trim_matches('"').to_string())
                        .unwrap_or_else(|| format!("Plot{}", plot_count + 1));
                    ir_body.push(IrStmt::SetBuffer(plot_count, IrExpr::IBars, expr));
                    meta.plots.push(PlotDef {
                        index: plot_count,
                        label,
                        draw_type: DrawType::Line,
                        color: "clrBlue".into(),
                        width: 1,
                        style: 0,
                    });
                    plot_count += 1;
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
            let next = trimmed[eq_pos + 1..].chars().next();
            if next == Some('=') {
                continue;
            }
            let lhs = trimmed[..eq_pos].trim();
            let rhs = trimmed[eq_pos + 1..].trim();
            if lhs.is_empty() || !lhs.chars().all(|c| c.is_alphanumeric() || c == '_') {
                continue;
            }

            // Special case: Param("label", default, ...) on the RHS → input
            if rhs.to_ascii_lowercase().starts_with("param(") && rhs.ends_with(')') {
                let inner = &rhs[6..rhs.len() - 1];
                let parts = split_top_level_commas(inner);
                let label = parts
                    .first()
                    .map(|s| s.trim().trim_matches('"').to_string())
                    .unwrap_or_else(|| lhs.to_string());
                let default_str = parts
                    .get(1)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_else(|| "0".into());
                let (ty, val) = if let Ok(i) = default_str.parse::<i32>() {
                    (IrType::I32, IrValue::I32(i))
                } else if let Ok(f) = default_str.parse::<f64>() {
                    (IrType::F64, IrValue::F64(f))
                } else {
                    (IrType::F64, IrValue::F64(0.0))
                };
                let param_type = match ty {
                    IrType::I32 => "int",
                    _ => "float",
                };
                inputs.push(IrInput {
                    name: lhs.to_ascii_lowercase(),
                    ir_type: ty,
                    default: val,
                });
                meta.inputs.push(InputParam {
                    name: label,
                    param_type: param_type.into(),
                    default_value: default_str,
                });
                locals.push((lhs.to_ascii_lowercase(), IrType::F64));
                continue;
            }

            if let Some(expr) = parse_afl_expr(rhs) {
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
    (ir_module, meta)
}

fn parse_afl_expr(expr: &str) -> Option<IrExpr> {
    let expr = expr.trim().trim_end_matches(';').trim();
    if expr.is_empty() {
        return None;
    }
    if expr.starts_with('(') && expr.ends_with(')') {
        return parse_afl_expr(&expr[1..expr.len() - 1]);
    }
    if let Ok(f) = expr.parse::<f64>() {
        if f.fract() == 0.0 && f.abs() < i32::MAX as f64 {
            return Some(IrExpr::I32Const(f as i32));
        }
        return Some(IrExpr::F64Const(f));
    }

    let lower = expr.to_ascii_lowercase();
    match lower.as_str() {
        "close" | "c" => return Some(IrExpr::IClose(Box::new(IrExpr::I32Const(0)))),
        "open" | "o" => return Some(IrExpr::IOpen(Box::new(IrExpr::I32Const(0)))),
        "high" | "h" => return Some(IrExpr::IHigh(Box::new(IrExpr::I32Const(0)))),
        "low" | "l" => return Some(IrExpr::ILow(Box::new(IrExpr::I32Const(0)))),
        "volume" | "v" => return Some(IrExpr::IVolume(Box::new(IrExpr::I32Const(0)))),
        "barindex" | "bar_index" => return Some(IrExpr::IBars),
        _ => {}
    }

    // Function call
    if let Some(open) = lower.find('(') {
        if expr.ends_with(')') {
            let func = &lower[..open];
            let args_str = &expr[open + 1..expr.len() - 1];
            let args_parts = split_top_level_commas(args_str);
            let ir_args: Option<Vec<IrExpr>> = args_parts
                .iter()
                .map(|a| parse_afl_expr(a.trim()))
                .collect();
            if let Some(ir_args) = ir_args {
                let mapped: Option<&str> = match func {
                    "ma" | "sma" | "average" | "movavg" => Some("ta_sma"),
                    "ema" | "expmovingavg" => Some("ta_ema"),
                    "rsi" => Some("ta_rsi"),
                    "atr" => Some("ta_atr"),
                    "hhv" | "highest" => Some("ta_highest"),
                    "llv" | "lowest" => Some("ta_lowest"),
                    "stdev" | "stddev" => Some("ta_stdev"),
                    "absvalue" | "abs" => Some("math_abs"),
                    "sqrt" => Some("math_sqrt"),
                    "log" => Some("math_log"),
                    "max" => Some("math_max"),
                    "min" => Some("math_min"),
                    _ => None,
                };
                if let Some(name) = mapped {
                    return Some(IrExpr::Call(name.into(), ir_args));
                }
                if func == "iif" && ir_args.len() == 3 {
                    return Some(IrExpr::Call("__select_f64".into(), ir_args));
                }
            }
        }
    }

    for op_str in &[
        " + ", " - ", " * ", " / ", " > ", " < ", " >= ", " <= ", " == ", " != ",
    ] {
        if let Some(pos) = expr.find(op_str) {
            let left = parse_afl_expr(&expr[..pos])?;
            let right = parse_afl_expr(&expr[pos + op_str.len()..])?;
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

    if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Some(IrExpr::GetLocal(lower));
    }
    None
}

fn strip_block_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'*') {
            chars.next();
            while let Some(d) = chars.next() {
                if d == '*' && chars.peek() == Some(&'/') {
                    chars.next();
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

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
    use crate::compile_afl;

    #[test]
    fn test_afl_simple_ma() {
        let src = r#"
_SECTION_BEGIN("EMA Cross");
Ema20 = EMA(Close, 20);
Plot(Ema20, "EMA20", colorBlue);
_SECTION_END();
"#;
        let result = compile_afl(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.short_name, "EMA Cross");
        assert_eq!(meta.plots.len(), 1);
    }

    #[test]
    fn test_afl_param_becomes_input() {
        let src = r#"
Length = Param("Length", 14, 2, 100, 1);
rsi = RSI(Length);
Plot(rsi, "RSI", colorYellow);
"#;
        let result = compile_afl(src);
        let meta = result.metadata.unwrap();
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.inputs[0].default_value, "14");
    }

    #[test]
    fn test_afl_multi_plot() {
        let src = r#"
Plot(Close, "C", colorWhite);
Plot(High, "H", colorGreen);
Plot(Low, "L", colorRed);
"#;
        let result = compile_afl(src);
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 3);
    }

    #[test]
    fn test_afl_shortcuts() {
        assert!(parse_afl_expr("C").is_some());
        assert!(parse_afl_expr("H").is_some());
        assert!(parse_afl_expr("L").is_some());
    }

    #[test]
    fn test_afl_hhv_llv_mapping() {
        if let Some(IrExpr::Call(name, _)) = parse_afl_expr("HHV(High, 20)") {
            assert_eq!(name, "ta_highest");
        } else {
            panic!("HHV should map to ta_highest");
        }
        if let Some(IrExpr::Call(name, _)) = parse_afl_expr("LLV(Low, 20)") {
            assert_eq!(name, "ta_lowest");
        } else {
            panic!("LLV should map to ta_lowest");
        }
    }

    #[test]
    fn test_afl_block_comment_stripped() {
        let src = r#"
/* this is a
   block comment */
Plot(Close, "C", colorBlue);
"#;
        let result = compile_afl(src);
        assert_eq!(result.metadata.unwrap().plots.len(), 1);
    }

    #[test]
    fn test_afl_empty_source() {
        let result = compile_afl("");
        assert!(result.metadata.is_some());
    }

    #[test]
    fn test_afl_case_insensitive() {
        let src = r#"
ema20 = ema(close, 20);
plot(ema20, "EMA", colorBlue);
"#;
        let result = compile_afl(src);
        assert_eq!(result.metadata.unwrap().plots.len(), 1);
    }

    #[test]
    fn test_afl_math_expression() {
        let src = r#"
mid = (High + Low) / 2;
Plot(mid, "Mid", colorBlue);
"#;
        let result = compile_afl(src);
        assert_eq!(result.metadata.unwrap().plots.len(), 1);
    }

    #[test]
    fn test_afl_iif_select_expression() {
        let src = r#"
fast = EMA(Close, 10);
slow = EMA(Close, 20);
trend = IIf(fast > slow, fast, slow);
Plot(trend, "Trend", colorGreen);
"#;
        let result = compile_afl(src);
        assert!(result.wasm.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 1);

        if let Some(IrExpr::Call(name, args)) = parse_afl_expr("IIf(Close > Open, High, Low)") {
            assert_eq!(name, "__select_f64");
            assert_eq!(args.len(), 3);
        } else {
            panic!("IIf should map to __select_f64");
        }
    }

    #[test]
    fn test_afl_section_name_extracted() {
        let src = r#"_SECTION_BEGIN("My Indicator");
Plot(Close, "C", colorBlue);
_SECTION_END();
"#;
        let result = compile_afl(src);
        assert_eq!(result.metadata.unwrap().short_name, "My Indicator");
    }
}
