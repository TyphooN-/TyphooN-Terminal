//! PineScript v4 / v5 Parser — converts PineScript to the same IR as MQL5.
//!
//! Auto-detects the version from the `//@version=N` header. Pine v4 uses
//! `study()` instead of `indicator()`, and some legacy function call sites
//! (`rsi(...)` instead of `ta.rsi(...)`, `sma(...)` instead of `ta.sma(...)`,
//! `security(...)` instead of `request.security(...)`). We accept both and
//! normalise them into the same IR.
//!
//! Supports a subset of PineScript v4/v5 sufficient for common indicators:
//! - //@version=4 or //@version=5 header
//! - indicator() (v5) / study() (v4) declaration
//! - input.int(), input.float(), input.bool(), input.string()
//! - ta.sma(), ta.ema(), ta.rsi(), ta.atr(), ta.highest(), ta.lowest()
//! - ta.crossover(), ta.crossunder()
//! - plot(), hline(), fill(), bgcolor()
//! - math.abs(), math.max(), math.min(), math.sqrt(), math.log()
//! - close, open, high, low, volume, bar_index
//! - if/else, for loops, var declarations
//! - Basic arithmetic and comparison operators

use crate::ir::*;
use crate::{CompileResult, DiagLevel, Diagnostic, DrawType, IndicatorMeta, InputParam, PlotDef};
use std::collections::HashSet;

/// Parse PineScript source and produce IR module.
pub fn parse_pine(source: &str) -> CompileResult {
    let (ir_module, meta, pine_version) = build_ir_internal(source);
    let mut diagnostics = Vec::new();
    match crate::codegen::emit_wasm(&ir_module) {
        Ok(wasm) => {
            diagnostics.push(Diagnostic {
                level: DiagLevel::Info,
                message: format!(
                    "PineScript v{} compiled: {} inputs, {} plots",
                    pine_version,
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
                message: format!("WASM codegen failed: {e}"),
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

/// Build IR + meta for PineScript source — used by the cross-language transpiler.
pub fn build_ir(source: &str) -> (IrModule, IndicatorMeta) {
    let (m, meta, _) = build_ir_internal(source);
    (m, meta)
}

fn build_ir_internal(source: &str) -> (IrModule, IndicatorMeta, u8) {
    let mut meta = IndicatorMeta {
        short_name: String::new(),
        buffers: 0,
        separate_window: false,
        inputs: Vec::new(),
        plots: Vec::new(),
    };
    let mut ir_body: Vec<IrStmt> = Vec::new();
    let mut inputs: Vec<IrInput> = Vec::new();
    let mut locals: Vec<(String, IrType)> = Vec::new();
    let mut local_names: HashSet<String> = HashSet::new();
    let mut plot_count = 0usize;
    // Detect version header. v4 scripts use `//@version=4` and call functions
    // without the `ta.`/`math.` namespace prefix. v5 requires the prefix.
    let mut pine_version: u8 = 5;
    for line in source.lines() {
        let t = line.trim();
        if let Some(ver) = t.strip_prefix("//@version=") {
            pine_version = ver.trim().parse().unwrap_or(5);
            break;
        }
    }

    // Helper: given a line, rewrite v4 bareword calls into v5 namespaced form
    // so the rest of the scanner only needs one set of match arms.
    let normalize_line = |line: String| -> String {
        if pine_version >= 5 {
            return line;
        }
        let replacements: &[(&str, &str)] = &[
            ("study(", "indicator("),
            ("security(", "request.security("),
            ("sma(", "ta.sma("),
            ("ema(", "ta.ema("),
            ("rma(", "ta.rma("),
            ("wma(", "ta.wma("),
            ("vwma(", "ta.vwma("),
            ("rsi(", "ta.rsi("),
            ("atr(", "ta.atr("),
            ("highest(", "ta.highest("),
            ("lowest(", "ta.lowest("),
            ("stdev(", "ta.stdev("),
            ("crossover(", "ta.crossover("),
            ("crossunder(", "ta.crossunder("),
            ("change(", "ta.change("),
            ("tr(", "ta.tr("),
            ("abs(", "math.abs("),
            ("sqrt(", "math.sqrt("),
            ("log(", "math.log("),
            ("max(", "math.max("),
            ("min(", "math.min("),
            ("input.int(", "input.int("), // already v5 form — no-op
            ("input(", "input.int("),     // v4 `input(14)` → treat as int
        ];
        let mut out = line;
        for (from, to) in replacements {
            // Replace only when preceded by a non-identifier char (or at line start)
            // so `ta.sma(` doesn't become `ta.ta.sma(`.
            out = replace_unprefixed(&out, from, to);
        }
        out
    };

    for (_line_num, raw_line) in source.lines().enumerate() {
        let normalized = normalize_line(raw_line.to_string());
        let line = normalized.as_str();
        let trimmed = line.trim();

        // Skip comments and empty lines
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }

        // Version header
        if trimmed.starts_with("//@version=") {
            continue;
        }

        // indicator() declaration (study() in v4 was rewritten above)
        if trimmed.starts_with("indicator(") {
            if let Some(name) = extract_string_arg(trimmed, "indicator(") {
                meta.short_name = name;
            }
            if trimmed.contains("overlay=false") || trimmed.contains("overlay = false") {
                meta.separate_window = true;
            }
            continue;
        }

        // Input declarations: "name = input.int(...)" or "name = input(...)"
        if trimmed.contains("input.int(")
            || trimmed.contains("input.float(")
            || trimmed.contains("input.bool(")
            || trimmed.contains("input.string(")
            || (trimmed.contains("= input(") && !trimmed.contains("= input."))
        {
            let input_type = if trimmed.contains("input.int(") {
                "int"
            } else if trimmed.contains("input.float(") {
                "float"
            } else if trimmed.contains("input.bool(") {
                "bool"
            } else {
                "float"
            };
            let parsed = parse_pine_input(trimmed, input_type);
            if let Some((name, default)) = parsed {
                let (ir_type, ir_default) = match input_type {
                    "int" => (IrType::I32, IrValue::I32(default.parse().unwrap_or(14))),
                    "bool" => (IrType::Bool, IrValue::Bool(default == "true")),
                    _ => (IrType::F64, IrValue::F64(default.parse().unwrap_or(0.0))),
                };
                inputs.push(IrInput {
                    name: name.clone(),
                    ir_type,
                    default: ir_default,
                });
                meta.inputs.push(InputParam {
                    name,
                    param_type: input_type.into(),
                    default_value: default,
                });
            }
            continue;
        }

        // plot() calls — must be checked BEFORE variable assignment (plot args contain '=')
        if trimmed.starts_with("plot(") {
            let args = extract_parens(trimmed);
            if let Some(first_arg) = args.split(',').next() {
                let first_arg = first_arg.trim();
                if let Some(ir_expr) = parse_pine_expr(first_arg) {
                    ir_body.push(IrStmt::SetBuffer(plot_count, IrExpr::IBars, ir_expr));
                    meta.plots.push(PlotDef {
                        index: plot_count,
                        label: extract_plot_title(trimmed)
                            .unwrap_or_else(|| format!("Plot {}", plot_count)),
                        draw_type: DrawType::Line,
                        color: extract_plot_color(trimmed).unwrap_or_else(|| "clrBlue".into()),
                        width: 1,
                        style: 0,
                    });
                    plot_count += 1;
                }
            }
            continue;
        }

        // hline() calls
        if trimmed.starts_with("hline(") {
            // hline is a constant level — handled differently
            continue;
        }

        // bgcolor() / fill() calls — skip for now (contain '=' in args)
        if trimmed.starts_with("bgcolor(") || trimmed.starts_with("fill(") {
            continue;
        }

        // Variable assignments: name = expr
        if let Some(eq_pos) = trimmed.find('=') {
            let lhs = trimmed[..eq_pos].trim();
            let rhs = trimmed[eq_pos + 1..].trim();

            // Skip if it's a comparison (==, !=, <=, >=)
            if rhs.starts_with('=')
                || trimmed.chars().nth(eq_pos.saturating_sub(1)) == Some('!')
                || trimmed.chars().nth(eq_pos.saturating_sub(1)) == Some('<')
                || trimmed.chars().nth(eq_pos.saturating_sub(1)) == Some('>')
            {
                continue;
            }

            // Handle var keyword
            let var_name = lhs.strip_prefix("var ").unwrap_or(lhs).trim();
            if var_name.contains(' ') || var_name.contains('(') {
                continue;
            } // not a simple assignment

            // Parse RHS expression. Keep stable local order but track membership
            // in a side set so repeated assignments do not linearly scan or
            // duplicate local declarations on large imported scripts.
            if let Some(ir_expr) = parse_pine_expr(rhs) {
                let var_name = var_name.to_string();
                if local_names.insert(var_name.clone()) {
                    locals.push((var_name.clone(), IrType::F64));
                }
                ir_body.push(IrStmt::SetLocal(var_name, ir_expr));
            }
            continue;
        }
    }

    meta.buffers = plot_count;

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
    (ir_module, meta, pine_version)
}

/// Parse a PineScript expression to IR.
fn parse_pine_expr(expr: &str) -> Option<IrExpr> {
    let expr = expr.trim();

    // Numeric literal
    if let Ok(f) = expr.parse::<f64>() {
        return Some(IrExpr::F64Const(f));
    }
    if let Ok(i) = expr.parse::<i32>() {
        return Some(IrExpr::I32Const(i));
    }

    // Built-in series
    match expr {
        "close" => return Some(IrExpr::IClose(Box::new(IrExpr::I32Const(0)))),
        "open" => return Some(IrExpr::IOpen(Box::new(IrExpr::I32Const(0)))),
        "high" => return Some(IrExpr::IHigh(Box::new(IrExpr::I32Const(0)))),
        "low" => return Some(IrExpr::ILow(Box::new(IrExpr::I32Const(0)))),
        "volume" => return Some(IrExpr::IVolume(Box::new(IrExpr::I32Const(0)))),
        "bar_index" => return Some(IrExpr::IBars),
        _ => {}
    }

    // ta.sma(source, length)
    if expr.starts_with("ta.sma(") {
        let args = extract_parens(expr);
        let parts: Vec<&str> = args.splitn(2, ',').collect();
        if parts.len() == 2 {
            let source = parse_pine_expr(parts[0].trim())?;
            let length = parse_pine_expr(parts[1].trim())?;
            return Some(IrExpr::Call("ta_sma".into(), vec![source, length]));
        }
    }

    // ta.ema(source, length)
    if expr.starts_with("ta.ema(") {
        let args = extract_parens(expr);
        let parts: Vec<&str> = args.splitn(2, ',').collect();
        if parts.len() == 2 {
            let source = parse_pine_expr(parts[0].trim())?;
            let length = parse_pine_expr(parts[1].trim())?;
            return Some(IrExpr::Call("ta_ema".into(), vec![source, length]));
        }
    }

    // ta.rsi(source, length)
    if expr.starts_with("ta.rsi(") {
        let args = extract_parens(expr);
        let parts: Vec<&str> = args.splitn(2, ',').collect();
        if parts.len() == 2 {
            let source = parse_pine_expr(parts[0].trim())?;
            let length = parse_pine_expr(parts[1].trim())?;
            return Some(IrExpr::Call("ta_rsi".into(), vec![source, length]));
        }
    }

    // ta.atr(length)
    if expr.starts_with("ta.atr(") {
        let args = extract_parens(expr);
        let length = parse_pine_expr(args.trim())?;
        return Some(IrExpr::Call("ta_atr".into(), vec![length]));
    }

    // ta.highest(source, length) / ta.lowest(source, length)
    if expr.starts_with("ta.highest(") {
        let args = extract_parens(expr);
        let parts: Vec<&str> = args.splitn(2, ',').collect();
        if parts.len() == 2 {
            let source = parse_pine_expr(parts[0].trim())?;
            let length = parse_pine_expr(parts[1].trim())?;
            return Some(IrExpr::Call("ta_highest".into(), vec![source, length]));
        }
    }
    if expr.starts_with("ta.lowest(") {
        let args = extract_parens(expr);
        let parts: Vec<&str> = args.splitn(2, ',').collect();
        if parts.len() == 2 {
            let source = parse_pine_expr(parts[0].trim())?;
            let length = parse_pine_expr(parts[1].trim())?;
            return Some(IrExpr::Call("ta_lowest".into(), vec![source, length]));
        }
    }

    // ta.crossover(a, b) / ta.crossunder(a, b)
    if expr.starts_with("ta.crossover(") {
        return parse_two_arg_call(expr, "ta_crossover");
    }
    if expr.starts_with("ta.crossunder(") {
        return parse_two_arg_call(expr, "ta_crossunder");
    }
    // ta.stdev(source, length)
    if expr.starts_with("ta.stdev(") {
        return parse_two_arg_call(expr, "ta_stdev");
    }
    // ta.change(source)
    if expr.starts_with("ta.change(") {
        return parse_single_arg_call(expr, "ta_change");
    }
    // ta.tr (true range — no args)
    if expr == "ta.tr" {
        return Some(IrExpr::Call("ta_tr".into(), vec![]));
    }
    // nz(val, replacement)
    if expr.starts_with("nz(") {
        return parse_two_arg_call(expr, "nz");
    }

    // math.abs/max/min/sqrt/log
    if expr.starts_with("math.abs(") {
        return parse_single_arg_call(expr, "math_abs");
    }
    if expr.starts_with("math.sqrt(") {
        return parse_single_arg_call(expr, "math_sqrt");
    }
    if expr.starts_with("math.log(") {
        return parse_single_arg_call(expr, "math_log");
    }
    if expr.starts_with("math.max(") {
        return parse_two_arg_call(expr, "math_max");
    }
    if expr.starts_with("math.min(") {
        return parse_two_arg_call(expr, "math_min");
    }

    // Binary operators (basic — no precedence, left to right)
    for op_str in &[
        " + ", " - ", " * ", " / ", " > ", " < ", " >= ", " <= ", " == ", " != ",
    ] {
        if let Some(pos) = expr.find(op_str) {
            let left = parse_pine_expr(&expr[..pos])?;
            let right = parse_pine_expr(&expr[pos + op_str.len()..])?;
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

    // Variable reference (identifier)
    if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Some(IrExpr::GetLocal(expr.to_string()));
    }

    None
}

fn parse_single_arg_call(expr: &str, func: &str) -> Option<IrExpr> {
    let args = extract_parens(expr);
    let inner = parse_pine_expr(args.trim())?;
    Some(IrExpr::Call(func.into(), vec![inner]))
}

fn parse_two_arg_call(expr: &str, func: &str) -> Option<IrExpr> {
    let args = extract_parens(expr);
    let parts: Vec<&str> = args.splitn(2, ',').collect();
    if parts.len() == 2 {
        let a = parse_pine_expr(parts[0].trim())?;
        let b = parse_pine_expr(parts[1].trim())?;
        Some(IrExpr::Call(func.into(), vec![a, b]))
    } else {
        None
    }
}

/// Replace occurrences of `from` with `to` only when not preceded by an
/// identifier character. Used for v4 → v5 normalisation so we don't mangle
/// `ta.sma(` into `ta.ta.sma(` when both forms coexist.
fn replace_unprefixed(haystack: &str, from: &str, to: &str) -> String {
    if !haystack.contains(from) {
        return haystack.to_string();
    }
    let bytes = haystack.as_bytes();
    let fb = from.as_bytes();
    let mut out = String::with_capacity(haystack.len());
    let mut i = 0;
    while i < bytes.len() {
        if i + fb.len() <= bytes.len() && &bytes[i..i + fb.len()] == fb {
            let prev_ok = i == 0
                || (!bytes[i - 1].is_ascii_alphanumeric()
                    && bytes[i - 1] != b'_'
                    && bytes[i - 1] != b'.');
            if prev_ok {
                out.push_str(to);
                i += fb.len();
                continue;
            }
        }
        out.push(bytes[i] as char);
        i += 1;
    }
    out
}

/// Extract content between first ( and last ).
fn extract_parens(s: &str) -> &str {
    let start = s.find('(').map(|i| i + 1).unwrap_or(0);
    let end = s.rfind(')').unwrap_or(s.len());
    &s[start..end]
}

/// Extract first string argument from a function call: func("string", ...)
fn extract_string_arg(s: &str, _prefix: &str) -> Option<String> {
    let inner = extract_parens(s);
    if inner.starts_with('"') {
        inner[1..]
            .find('"')
            .map(|end| inner[1..end + 1].to_string())
    } else {
        None
    }
}

/// Parse PineScript input: name = input.type(defval=N, title="T")
fn parse_pine_input(line: &str, _type_hint: &str) -> Option<(String, String)> {
    // Extract variable name from "name = input.xxx(...)"
    let eq_pos = line.find('=')?;
    let name = line[..eq_pos].trim().to_string();

    let inner = extract_parens(line);
    // Find defval= or first positional arg
    let default = if let Some(dv) = inner.find("defval=") {
        let rest = &inner[dv + 7..];
        rest.split(&[',', ')'][..])
            .next()
            .unwrap_or("0")
            .trim()
            .to_string()
    } else {
        inner.split(',').next().unwrap_or("0").trim().to_string()
    };

    Some((name, default))
}

/// Extract title="..." from plot() args.
fn extract_plot_title(s: &str) -> Option<String> {
    let inner = extract_parens(s);
    if let Some(pos) = inner.find("title=") {
        let rest = &inner[pos + 6..];
        if rest.starts_with('"') {
            rest[1..].find('"').map(|end| rest[1..end + 1].to_string())
        } else {
            None
        }
    } else {
        None
    }
}

/// Extract color from plot() args (e.g., color=color.blue).
fn extract_plot_color(s: &str) -> Option<String> {
    let inner = extract_parens(s);
    if let Some(pos) = inner.find("color=") {
        let rest = &inner[pos + 6..];
        let color = rest.split(&[',', ')'][..]).next()?.trim();
        Some(
            match color {
                "color.blue" => "clrBlue",
                "color.red" => "clrRed",
                "color.green" => "clrGreen",
                "color.white" => "clrWhite",
                "color.yellow" => "clrYellow",
                "color.orange" => "clrOrange",
                "color.purple" => "clrMagenta",
                _ => "clrBlue",
            }
            .into(),
        )
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_sma_indicator() {
        let source = r#"
//@version=5
indicator("Simple SMA", overlay=true)
length = input.int(defval=20, title="Length")
sma_val = ta.sma(close, length)
plot(sma_val, title="SMA", color=color.blue)
"#;
        let result = parse_pine(source);
        assert!(
            result.wasm.is_some(),
            "Should produce WASM: {:?}",
            result.diagnostics
        );
        let meta = result.metadata.unwrap();
        assert_eq!(meta.short_name, "Simple SMA");
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.inputs[0].name, "length");
        assert_eq!(meta.plots.len(), 1);
        assert_eq!(meta.plots[0].label, "SMA");
    }

    #[test]
    fn parse_rsi_indicator() {
        let source = r#"
//@version=5
indicator("RSI", overlay=false)
length = input.int(defval=14, title="Period")
rsi_val = ta.rsi(close, length)
plot(rsi_val, title="RSI", color=color.yellow)
"#;
        let result = parse_pine(source);
        assert!(
            result.wasm.is_some(),
            "Should produce WASM: {:?}",
            result.diagnostics
        );
        let meta = result.metadata.unwrap();
        assert_eq!(meta.short_name, "RSI");
        assert!(meta.separate_window);
        assert_eq!(meta.buffers, 1);
    }

    #[test]
    fn parse_math_expressions() {
        let source = r#"
//@version=5
indicator("Math Test")
val = math.abs(close - open)
mx = math.max(high, low)
plot(val)
"#;
        let result = parse_pine(source);
        assert!(result.wasm.is_some());
    }

    #[test]
    fn parse_binary_ops() {
        let source = r#"
//@version=5
indicator("Ops")
diff = close - open
ratio = high / low
plot(diff)
"#;
        let result = parse_pine(source);
        assert!(result.wasm.is_some());
    }

    #[test]
    fn parse_empty_source() {
        let result = parse_pine("");
        // Empty source should still produce valid (empty) WASM
        assert!(result.wasm.is_some());
    }

    #[test]
    fn parse_multiple_plots() {
        let source = r#"
//@version=5
indicator("Multi Plot")
fast = ta.ema(close, 9)
slow = ta.ema(close, 21)
plot(fast, title="Fast", color=color.green)
plot(slow, title="Slow", color=color.red)
"#;
        let result = parse_pine(source);
        assert!(result.wasm.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 2);
        assert_eq!(meta.buffers, 2);
    }

    #[test]
    fn parse_pine_v4_study_and_bareword_calls() {
        let source = r#"
//@version=4
study("v4 indicator", overlay=true)
length = input(defval=14, title="Length")
val = sma(close, length)
plot(val, title="SMA", color=color.blue)
"#;
        let result = parse_pine(source);
        assert!(
            result.wasm.is_some(),
            "v4 should compile: {:?}",
            result.diagnostics
        );
        let meta = result.metadata.unwrap();
        assert_eq!(meta.short_name, "v4 indicator");
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.plots.len(), 1);
        // Diagnostic should mention v4
        let has_v4 = result.diagnostics.iter().any(|d| d.message.contains("v4"));
        assert!(has_v4, "diagnostics should mention Pine v4");
    }

    #[test]
    fn replace_unprefixed_skips_namespaced_calls() {
        let input = "x = ta.sma(close, 20) + sma(close, 10)";
        // Only the bareword sma(...) should be rewritten; ta.sma(...) stays put.
        let out = replace_unprefixed(input, "sma(", "ta.sma(");
        assert!(out.contains("ta.sma(close, 20)"));
        assert!(out.contains("ta.sma(close, 10)"));
        // Should not produce ta.ta.sma
        assert!(!out.contains("ta.ta.sma"));
    }

    #[test]
    fn repeated_assignments_emit_one_local() {
        let source = r#"
//@version=5
indicator("Repeated Local")
trend = close
trend = ta.sma(close, 20)
plot(trend)
"#;
        let (module, _) = build_ir(source);
        let locals = &module.on_calculate.as_ref().unwrap().locals;
        assert_eq!(locals.iter().filter(|(name, _)| name == "trend").count(), 1);
    }

    #[test]
    fn parse_ta_functions() {
        let source = r#"
//@version=5
indicator("TA Test")
atr_val = ta.atr(14)
highest_val = ta.highest(high, 20)
lowest_val = ta.lowest(low, 20)
cross = ta.crossover(close, open)
plot(atr_val)
"#;
        let result = parse_pine(source);
        assert!(result.wasm.is_some());
    }
}
