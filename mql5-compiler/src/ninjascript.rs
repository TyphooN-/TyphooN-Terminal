//! NinjaScript (NinjaTrader) indicator-only subset frontend.
//!
//! NinjaScript is full C# — a proper parser would be a major undertaking.
//! This frontend handles the declarative portion common to community
//! indicators: property-based `[NinjaScriptProperty]` inputs, `AddPlot(...)`
//! calls in `OnStateChange()`, and assignment statements inside
//! `OnBarUpdate()`.
//!
//! Supported:
//! - `[NinjaScriptProperty]` attribute followed by `public int|double Name { get; set; } = default;`
//! - `AddPlot(Brushes.X, "Label")` inside `OnStateChange` — one plot per call
//! - Assignment statements inside `OnBarUpdate()`: `Value[0] = expr;` / `Plot0[0] = expr;` / `SomePlot[0] = expr;`
//! - Built-in series: `Close[0]`, `Open[0]`, `High[0]`, `Low[0]`, `Volume[0]`
//! - Built-in indicator functions: `SMA(src, period)[0]`, `EMA(...)[0]`, `RSI(...)[0]`, `ATR(period)[0]`,
//!   `MAX(src, period)[0]`, `MIN(src, period)[0]`, `StdDev(src, period)[0]`
//! - `Math.Abs/Sqrt/Log/Max/Min(...)`
//! - `//` and `/* ... */` comments
//!
//! Not supported (indicator subset — strategies out of scope):
//! - `OnStartUp` / `OnTermination` full C# flow
//! - `if`/`for`/`while` blocks (non-trivial without a real C# parser)
//! - User-defined classes, enums, LINQ
//! - DataSeries arithmetic outside `[0]` indexing

use crate::ir::*;
use crate::{CompileResult, DiagLevel, Diagnostic, DrawType, IndicatorMeta, InputParam, PlotDef};
use std::collections::HashSet;

pub fn parse_ninjascript(source: &str) -> CompileResult {
    let (ir_module, meta) = build_ir(source);
    let mut diagnostics = Vec::new();
    match crate::codegen::emit_wasm(&ir_module) {
        Ok(wasm) => {
            diagnostics.push(Diagnostic {
                level: DiagLevel::Info,
                message: format!(
                    "NinjaScript compiled: {} inputs, {} plots",
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
                message: format!("NinjaScript codegen failed: {e}"),
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
        short_name: String::from("NinjaScript"),
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

    let cleaned = strip_comments(source);
    let lines: Vec<&str> = cleaned.lines().collect();

    // Extract the indicator class name for short_name.
    for line in &lines {
        let t = line.trim();
        if let Some(pos) = t.find("class ") {
            let rest = &t[pos + 6..];
            let name: String = rest
                .chars()
                .take_while(|c| c.is_alphanumeric() || *c == '_')
                .collect();
            if !name.is_empty() {
                meta.short_name = name;
                break;
            }
        }
    }

    // IsOverlay=false → separate window; default overlay.
    if cleaned.contains("IsOverlay = false") || cleaned.contains("IsOverlay=false") {
        meta.separate_window = true;
    }

    // Scan for `[NinjaScriptProperty]` followed by a `public TYPE Name { get; set; } = default;`
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();
        if line.starts_with("[NinjaScriptProperty") {
            // look at next non-empty line for the property declaration
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() {
                if let Some((name, ty, default)) = parse_csharp_property(lines[j].trim()) {
                    let (ir_ty, val) = match ty.as_str() {
                        "int" => (IrType::I32, IrValue::I32(default.parse().unwrap_or(14))),
                        "bool" => (IrType::Bool, IrValue::Bool(default == "true")),
                        _ => (IrType::F64, IrValue::F64(default.parse().unwrap_or(0.0))),
                    };
                    inputs.push(IrInput {
                        name: name.to_ascii_lowercase(),
                        ir_type: ir_ty,
                        default: val,
                    });
                    meta.inputs.push(InputParam {
                        name: name.clone(),
                        param_type: ty,
                        default_value: default,
                    });
                    let local_name = name.to_ascii_lowercase();
                    if local_names.insert(local_name.clone()) {
                        locals.push((local_name, IrType::F64));
                    }
                }
            }
            i = j + 1;
            continue;
        }

        // AddPlot(..., "Label") → register a plot slot
        if line.starts_with("AddPlot(") {
            let args = extract_parens_balanced(line);
            let parts = split_top_level_commas(&args);
            // Label is the last quoted string arg, or last arg
            let label = parts
                .iter()
                .rev()
                .filter_map(|p| {
                    let t = p.trim();
                    if t.starts_with('"') && t.ends_with('"') && t.len() >= 2 {
                        Some(t[1..t.len() - 1].to_string())
                    } else {
                        None
                    }
                })
                .next()
                .unwrap_or_else(|| format!("Plot{}", plot_count));
            meta.plots.push(PlotDef {
                index: plot_count,
                label,
                draw_type: DrawType::Line,
                color: "clrBlue".into(),
                width: 1,
                style: 0,
            });
            plot_count += 1;
            i += 1;
            continue;
        }

        // Plot assignment inside a method: `Value[0] = expr;` / `Values[0][0] = expr;` / `Plot0[0] = expr;`
        if let Some((ir_expr, plot_idx)) = try_parse_plot_assignment(line) {
            // Auto-register a plot slot if AddPlot wasn't seen
            if plot_idx >= meta.plots.len() {
                meta.plots.push(PlotDef {
                    index: plot_idx,
                    label: format!("Plot{}", plot_idx),
                    draw_type: DrawType::Line,
                    color: "clrBlue".into(),
                    width: 1,
                    style: 0,
                });
            }
            ir_body.push(IrStmt::SetBuffer(plot_idx, IrExpr::IBars, ir_expr));
            i += 1;
            continue;
        }

        // Local assignment: `double x = expr;` or `x = expr;`
        if let Some((name, rhs)) = try_parse_local_assignment(line) {
            if let Some(e) = parse_ns_expr(&rhs) {
                let lname = name.to_ascii_lowercase();
                if local_names.insert(lname.clone()) {
                    locals.push((lname.clone(), IrType::F64));
                }
                ir_body.push(IrStmt::SetLocal(lname, e));
            }
        }

        i += 1;
    }

    meta.buffers = meta.plots.len().max(plot_count);
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

/// Parse `public int Length { get; set; } = 14;` → ("Length", "int", "14")
fn parse_csharp_property(line: &str) -> Option<(String, String, String)> {
    let line = line.trim().trim_end_matches(';');
    let after_public = line.strip_prefix("public ")?;
    let mut parts = after_public.split_whitespace();
    let ty = parts.next()?.to_string();
    let name = parts.next()?.to_string();
    // After the name there should be `{ get; set; } = default`
    let eq_pos = line.find('=')?;
    let default = line[eq_pos + 1..]
        .trim()
        .trim_end_matches(';')
        .trim()
        .to_string();
    Some((name, ty, default))
}

/// Returns Some((ir_expr, plot_index)) if the line looks like a plot assignment
/// (`Value[0] = expr;`, `Values[idx][0] = expr;`, or `SomePlotName[0] = expr;`).
fn try_parse_plot_assignment(line: &str) -> Option<(IrExpr, usize)> {
    let line = line.trim().trim_end_matches(';');
    let eq_pos = line.find('=')?;
    // Must not be ==, !=, <=, >=
    let prev = line[..eq_pos].chars().last();
    if matches!(prev, Some('!' | '<' | '>' | '=')) {
        return None;
    }
    let lhs = line[..eq_pos].trim();
    let rhs = line[eq_pos + 1..].trim();

    // Accept `Value[0] = ...` (NinjaScript default plot buffer)
    // or `Values[N][0] = ...` (named plot by slot)
    // or `<Ident>[0] = ...` where Ident looks like a plot name
    let first_bracket = lhs.find('[')?;
    let prefix = &lhs[..first_bracket];

    let plot_idx = if prefix == "Values" {
        // Parse `Values[N][0]`
        let rest = &lhs[first_bracket + 1..];
        let close = rest.find(']')?;
        rest[..close].trim().parse::<usize>().ok()?
    } else if prefix == "Value" || !prefix.is_empty() {
        // Single-plot form
        0
    } else {
        return None;
    };

    let expr = parse_ns_expr(rhs)?;
    Some((expr, plot_idx))
}

fn try_parse_local_assignment(line: &str) -> Option<(String, String)> {
    let line = line.trim().trim_end_matches(';');
    if line.is_empty() {
        return None;
    }
    // Skip lines that are control flow, method headers, brackets
    if line.starts_with("if")
        || line.starts_with("for")
        || line.starts_with("while")
        || line.starts_with("return")
        || line.starts_with("//")
        || line.starts_with('{')
        || line.starts_with('}')
        || line.starts_with("public")
        || line.starts_with("private")
        || line.starts_with("protected")
    {
        return None;
    }

    let eq_pos = line.find('=')?;
    let prev = line[..eq_pos].chars().last();
    if matches!(prev, Some('!' | '<' | '>' | '=')) {
        return None;
    }
    let lhs = line[..eq_pos].trim();
    let rhs = line[eq_pos + 1..].trim().to_string();

    // Strip an optional type annotation: `double x` → `x`
    let name = if let Some(pos) = lhs.rfind(char::is_whitespace) {
        lhs[pos + 1..].to_string()
    } else {
        lhs.to_string()
    };
    // Name must be identifier-like, no brackets
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    Some((name, rhs))
}

/// Parse a NinjaScript expression. Handles `Close[0]`, `SMA(Close, 20)[0]`,
/// numeric literals, `Math.X(...)`, and basic arithmetic.
fn parse_ns_expr(expr: &str) -> Option<IrExpr> {
    let expr = expr.trim().trim_end_matches(';').trim();
    if expr.is_empty() {
        return None;
    }
    if expr.starts_with('(') && expr.ends_with(')') && matched_parens(expr) {
        return parse_ns_expr(&expr[1..expr.len() - 1]);
    }
    if let Ok(f) = expr.parse::<f64>() {
        if f.fract() == 0.0 && f.abs() < i32::MAX as f64 {
            return Some(IrExpr::I32Const(f as i32));
        }
        return Some(IrExpr::F64Const(f));
    }

    // Strip trailing [0] (current-bar indexing) — we treat any bar index as current.
    let expr_core = if let Some(last_open) = expr.rfind('[') {
        if expr.ends_with(']') {
            let inner = &expr[last_open + 1..expr.len() - 1];
            if inner.trim() == "0" {
                &expr[..last_open]
            } else {
                expr
            }
        } else {
            expr
        }
    } else {
        expr
    };

    // Built-in series
    match expr_core.trim() {
        "Close" => return Some(IrExpr::IClose(Box::new(IrExpr::I32Const(0)))),
        "Open" => return Some(IrExpr::IOpen(Box::new(IrExpr::I32Const(0)))),
        "High" => return Some(IrExpr::IHigh(Box::new(IrExpr::I32Const(0)))),
        "Low" => return Some(IrExpr::ILow(Box::new(IrExpr::I32Const(0)))),
        "Volume" => return Some(IrExpr::IVolume(Box::new(IrExpr::I32Const(0)))),
        "CurrentBar" => return Some(IrExpr::IBars),
        _ => {}
    }

    // Function call: `Name(args)` or `Math.Func(args)`
    if let Some(open) = expr_core.find('(') {
        if expr_core.ends_with(')') {
            let func_raw = expr_core[..open].trim();
            // Math.Abs etc.
            let func = func_raw.strip_prefix("Math.").unwrap_or(func_raw);
            let args_str = &expr_core[open + 1..expr_core.len() - 1];
            let parts = split_top_level_commas(args_str);
            let ir_args: Option<Vec<IrExpr>> =
                parts.iter().map(|a| parse_ns_expr(a.trim())).collect();
            if let Some(ir_args) = ir_args {
                let mapped: Option<&str> = match func {
                    "SMA" | "Sma" => Some("ta_sma"),
                    "EMA" | "Ema" => Some("ta_ema"),
                    "RSI" | "Rsi" => Some("ta_rsi"),
                    "ATR" | "Atr" => Some("ta_atr"),
                    "MAX" => Some("ta_highest"),
                    "MIN" => Some("ta_lowest"),
                    "StdDev" | "STDDEV" => Some("ta_stdev"),
                    "Abs" => Some("math_abs"),
                    "Sqrt" => Some("math_sqrt"),
                    "Log" => Some("math_log"),
                    "Max" => Some("math_max"),
                    "Min" => Some("math_min"),
                    _ => None,
                };
                if let Some(name) = mapped {
                    return Some(IrExpr::Call(name.into(), ir_args));
                }
            }
        }
    }

    // Arithmetic and comparison
    for op_str in &[
        " + ", " - ", " * ", " / ", " >= ", " <= ", " > ", " < ", " == ", " != ",
    ] {
        if let Some(pos) = expr_core.find(op_str) {
            let left = parse_ns_expr(&expr_core[..pos])?;
            let right = parse_ns_expr(&expr_core[pos + op_str.len()..])?;
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

    // Bare identifier
    if expr_core.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Some(IrExpr::GetLocal(expr_core.to_ascii_lowercase()));
    }
    None
}

fn matched_parens(s: &str) -> bool {
    let mut depth = 0i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 && i < s.len() - 1 {
                    return false;
                }
            }
            _ => {}
        }
    }
    depth == 0
}

fn extract_parens_balanced(s: &str) -> String {
    let start = match s.find('(') {
        Some(i) => i + 1,
        None => return String::new(),
    };
    let mut depth = 1i32;
    let mut end = start;
    for (i, c) in s[start..].char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = start + i;
                    break;
                }
            }
            _ => {}
        }
    }
    s[start..end].to_string()
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

fn strip_comments(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'/') {
            // skip to EOL
            while let Some(&d) = chars.peek() {
                chars.next();
                if d == '\n' {
                    out.push('\n');
                    break;
                }
            }
        } else if c == '/' && chars.peek() == Some(&'*') {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_ninjascript;

    #[test]
    fn test_ns_simple_ema() {
        let src = r#"
public class MyIndicator : Indicator
{
    [NinjaScriptProperty]
    public int Period { get; set; } = 20;

    protected override void OnStateChange()
    {
        if (State == State.SetDefaults)
        {
            AddPlot(Brushes.Blue, "EMA");
        }
    }

    protected override void OnBarUpdate()
    {
        Value[0] = EMA(Close, Period)[0];
    }
}
"#;
        let result = compile_ninjascript(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.short_name, "MyIndicator");
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.inputs[0].name, "Period");
        assert_eq!(meta.inputs[0].default_value, "20");
        assert_eq!(meta.plots.len(), 1);
        assert_eq!(meta.plots[0].label, "EMA");
    }

    #[test]
    fn test_ns_multi_plot() {
        let src = r#"
public class XYZ : Indicator
{
    protected override void OnStateChange()
    {
        AddPlot(Brushes.Green, "Fast");
        AddPlot(Brushes.Red, "Slow");
    }
    protected override void OnBarUpdate()
    {
        Values[0][0] = SMA(Close, 10)[0];
        Values[1][0] = SMA(Close, 20)[0];
    }
}
"#;
        let result = compile_ninjascript(src);
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 2);
        assert_eq!(meta.plots[0].label, "Fast");
        assert_eq!(meta.plots[1].label, "Slow");
    }

    #[test]
    fn test_ns_is_overlay_false_sets_separate_window() {
        let src = r#"
public class R : Indicator {
    protected override void OnStateChange() {
        if (State == State.SetDefaults) {
            IsOverlay = false;
            AddPlot(Brushes.Yellow, "RSI");
        }
    }
    protected override void OnBarUpdate() {
        Value[0] = RSI(Close, 14)[0];
    }
}
"#;
        let result = compile_ninjascript(src);
        assert!(result.metadata.unwrap().separate_window);
    }

    #[test]
    fn test_ns_comment_stripping() {
        let src = r#"
// this is a line comment
/* and this is a
   block comment */
public class X : Indicator {}
"#;
        let result = compile_ninjascript(src);
        assert_eq!(result.metadata.unwrap().short_name, "X");
    }

    #[test]
    fn test_ns_math_abs() {
        if let Some(IrExpr::Call(name, _)) = parse_ns_expr("Math.Abs(Close - Open)") {
            assert_eq!(name, "math_abs");
        } else {
            panic!("Math.Abs should map to math_abs");
        }
    }

    #[test]
    fn test_ns_parse_csharp_property() {
        let (n, t, d) = parse_csharp_property("public int Period { get; set; } = 14;").unwrap();
        assert_eq!(n, "Period");
        assert_eq!(t, "int");
        assert_eq!(d, "14");
    }

    #[test]
    fn test_ns_parse_csharp_property_double() {
        let (_, t, d) =
            parse_csharp_property("public double Multiplier { get; set; } = 2.5;").unwrap();
        assert_eq!(t, "double");
        assert_eq!(d, "2.5");
    }

    #[test]
    fn test_ns_empty_source() {
        let result = compile_ninjascript("");
        assert!(result.metadata.is_some());
    }

    #[test]
    fn test_ns_close_shortcut_with_bar_index() {
        // Close[0] should parse as current-bar close
        assert!(parse_ns_expr("Close[0]").is_some());
    }

    #[test]
    fn test_ns_sma_call() {
        if let Some(IrExpr::Call(name, args)) = parse_ns_expr("SMA(Close, 20)") {
            assert_eq!(name, "ta_sma");
            assert_eq!(args.len(), 2);
        } else {
            panic!("SMA should map to ta_sma");
        }
    }
    #[test]
    fn test_ns_duplicate_local_declared_once() {
        let src = r#"
public class Dup : Indicator {
    protected override void OnBarUpdate() {
        double value = Close[0];
        value = Open[0];
        Value[0] = value;
    }
}
"#;
        let (module, _) = build_ir(src);
        let locals = &module.on_calculate.as_ref().unwrap().locals;
        assert_eq!(locals.iter().filter(|(name, _)| name == "value").count(), 1);
    }
}
