//! cTrader cAlgo (Spotware cTrader) indicator-only subset frontend.
//!
//! cAlgo is full C# like NinjaScript, but with different attribute +
//! method conventions. We extract the declarative portion:
//!
//! - `[Indicator(...)]` class-level attribute (short_name, IsOverlay)
//! - `[Parameter("Label", DefaultValue = N)]` on `public int|double Foo { get; set; }`
//! - `[Output("Label")]` on `public IndicatorDataSeries Name { get; set; }` — becomes a plot slot
//! - Assignment statements inside `Calculate(int index)`:
//!     `Name[index] = expr;` (where Name is an Output or `Result[index] = expr;`)
//! - Built-in series: `Bars.ClosePrices[index]`, `MarketSeries.Close[index]`, etc.
//!   Both shortened `Close[index]` / `Open[index]` / ... also accepted.
//! - Built-in indicators: `Indicators.SimpleMovingAverage(source, periods).Result[index]`,
//!   `Indicators.ExponentialMovingAverage(...)`, `Indicators.RelativeStrengthIndex(...)`
//! - `Math.Abs/Sqrt/Log/Max/Min(...)`
//!
//! Not supported:
//! - `Robot` (cBot) base class — strategies out of scope
//! - Cross-timeframe (`MarketData.GetSeries(TimeFrame.H1)`) — mapped to current timeframe
//! - C# `if`/`foreach`/`LINQ`
//! - Nested `#region`/`#endregion` (stripped)

use crate::ir::*;
use crate::{CompileResult, DiagLevel, Diagnostic, DrawType, IndicatorMeta, InputParam, PlotDef};
use std::collections::{HashMap, HashSet};

pub fn parse_calgo(source: &str) -> CompileResult {
    let (ir_module, meta) = build_ir(source);
    let mut diagnostics = Vec::new();
    match crate::codegen::emit_wasm(&ir_module) {
        Ok(wasm) => {
            diagnostics.push(Diagnostic {
                level: DiagLevel::Info,
                message: format!(
                    "cAlgo compiled: {} inputs, {} plots",
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
                message: format!("cAlgo codegen failed: {e}"),
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
        short_name: String::from("cAlgo"),
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
    let mut output_name_to_idx: HashMap<String, usize> = HashMap::new();

    let cleaned = strip_comments_and_regions(source);
    let lines: Vec<&str> = cleaned.lines().collect();

    // Class name
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

    // [Indicator(IsOverlay = false, ...)]
    if cleaned.contains("IsOverlay = false") || cleaned.contains("IsOverlay=false") {
        meta.separate_window = true;
    }

    // Extract a short name from [Indicator(Name = "Foo", ...)] if present.
    // We MUST limit the search to the balanced parens of the [Indicator(...)]
    // attribute — otherwise we'll accidentally pick up the first quoted string
    // from a later [Parameter("Label", ...)] attribute.
    if let Some(pos) = cleaned.find("[Indicator(") {
        let attr = extract_parens_balanced(&cleaned[pos..]);
        if let Some(name_pos) = attr.find("\"") {
            let after = &attr[name_pos + 1..];
            if let Some(end) = after.find('"') {
                let extracted = &after[..end];
                if !extracted.is_empty() && extracted.len() < 64 {
                    meta.short_name = extracted.to_string();
                }
            }
        }
    }

    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim();

        // [Parameter("Label", DefaultValue = N)]
        if line.starts_with("[Parameter") {
            // Extract label and default
            let attr_inner = extract_parens_balanced(line);
            let label = extract_first_quoted(&attr_inner).unwrap_or_else(|| "Param".into());
            let default =
                extract_named_arg(&attr_inner, "DefaultValue").unwrap_or_else(|| "0".into());
            // Next non-empty line should be the property
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() {
                if let Some((name, ty)) = parse_calgo_property(lines[j].trim()) {
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
                        name: label,
                        param_type: ty,
                        default_value: default,
                    });
                    let local_name = name.to_ascii_lowercase();
                    if local_names.insert(local_name.clone()) {
                        locals.push((local_name, IrType::F64));
                    }
                }
                i = j + 1;
                continue;
            }
        }

        // [Output("Label", ...)]
        if line.starts_with("[Output") {
            let attr_inner = extract_parens_balanced(line);
            let label = extract_first_quoted(&attr_inner)
                .unwrap_or_else(|| format!("Output{}", plot_count));
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() {
                if let Some((name, _ty)) = parse_calgo_property(lines[j].trim()) {
                    output_name_to_idx.insert(name, plot_count);
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
                i = j + 1;
                continue;
            }
        }

        // Plot assignment: `<OutputName>[index] = expr;` or `Result[index] = expr;`
        if let Some((plot_idx, rhs)) = try_parse_output_assignment(line, &output_name_to_idx) {
            if let Some(e) = parse_calgo_expr(&rhs) {
                // Auto-register if user never used [Output]
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
                ir_body.push(IrStmt::SetBuffer(plot_idx, IrExpr::IBars, e));
            }
            i += 1;
            continue;
        }

        // Local assignment
        if let Some((name, rhs)) = try_parse_calgo_local(line) {
            if let Some(e) = parse_calgo_expr(&rhs) {
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

/// Parse `public int Period { get; set; }` → ("Period", "int").
/// Default value is held on the `[Parameter]` attribute, not the property.
fn parse_calgo_property(line: &str) -> Option<(String, String)> {
    let line = line.trim().trim_end_matches(';');
    let after_public = line.strip_prefix("public ")?;
    let mut parts = after_public.split_whitespace();
    let ty = parts.next()?.to_string();
    let name = parts.next()?.to_string();
    // cAlgo uses `IndicatorDataSeries` as the Output type — treat it as f64
    let ty_norm = if ty == "IndicatorDataSeries" {
        "double".to_string()
    } else {
        ty
    };
    Some((name, ty_norm))
}

fn try_parse_output_assignment(
    line: &str,
    output_name_to_idx: &HashMap<String, usize>,
) -> Option<(usize, String)> {
    let line = line.trim().trim_end_matches(';');
    let eq_pos = line.find('=')?;
    let prev = line[..eq_pos].chars().last();
    if matches!(prev, Some('!' | '<' | '>' | '=')) {
        return None;
    }
    let lhs = line[..eq_pos].trim();
    let rhs = line[eq_pos + 1..].trim().to_string();

    let first_bracket = lhs.find('[')?;
    let prefix = lhs[..first_bracket].trim();
    // Accept known output names, or "Result" (single-output convention)
    let plot_idx = if prefix == "Result" {
        0
    } else if let Some(&idx) = output_name_to_idx.get(prefix) {
        idx
    } else {
        return None;
    };
    Some((plot_idx, rhs))
}

fn try_parse_calgo_local(line: &str) -> Option<(String, String)> {
    let line = line.trim().trim_end_matches(';');
    if line.is_empty() {
        return None;
    }
    if line.starts_with("if")
        || line.starts_with("for")
        || line.starts_with("while")
        || line.starts_with("return")
        || line.starts_with('{')
        || line.starts_with('}')
        || line.starts_with("public")
        || line.starts_with("private")
        || line.starts_with("protected")
        || line.starts_with('[')
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
    // Disallow assignments to something[index]
    if lhs.contains('[') {
        return None;
    }
    let name = if let Some(pos) = lhs.rfind(char::is_whitespace) {
        lhs[pos + 1..].to_string()
    } else {
        lhs.to_string()
    };
    if name.is_empty() || !name.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return None;
    }
    Some((name, rhs))
}

fn parse_calgo_expr(expr: &str) -> Option<IrExpr> {
    let expr = expr.trim().trim_end_matches(';').trim();
    if expr.is_empty() {
        return None;
    }
    if expr.starts_with('(') && expr.ends_with(')') && matched_parens(expr) {
        return parse_calgo_expr(&expr[1..expr.len() - 1]);
    }
    if let Ok(f) = expr.parse::<f64>() {
        if f.fract() == 0.0 && f.abs() < i32::MAX as f64 {
            return Some(IrExpr::I32Const(f as i32));
        }
        return Some(IrExpr::F64Const(f));
    }

    // Strip trailing `[index]` / `[i]` — we treat any index as current bar.
    let expr_core = if let Some(last_open) = expr.rfind('[') {
        if expr.ends_with(']') {
            &expr[..last_open]
        } else {
            expr
        }
    } else {
        expr
    };
    let expr_core = expr_core.trim();

    // Built-in series: accept short + long forms.
    // Long: `Bars.ClosePrices`, `MarketSeries.Close`
    let normalized = expr_core
        .trim_start_matches("Bars.")
        .trim_start_matches("MarketSeries.");
    let cleaned = match normalized {
        "ClosePrices" | "Close" => return Some(IrExpr::IClose(Box::new(IrExpr::I32Const(0)))),
        "OpenPrices" | "Open" => return Some(IrExpr::IOpen(Box::new(IrExpr::I32Const(0)))),
        "HighPrices" | "High" => return Some(IrExpr::IHigh(Box::new(IrExpr::I32Const(0)))),
        "LowPrices" | "Low" => return Some(IrExpr::ILow(Box::new(IrExpr::I32Const(0)))),
        "TickVolumes" | "Volume" => return Some(IrExpr::IVolume(Box::new(IrExpr::I32Const(0)))),
        other => other,
    };

    // `Indicators.SimpleMovingAverage(src, period).Result` (with [i] already stripped)
    if let Some(dot) = cleaned.rfind('.') {
        let before = &cleaned[..dot];
        let after = &cleaned[dot + 1..];
        if after == "Result" || after == "Result " {
            // Recurse into the call portion
            return parse_calgo_expr(before);
        }
    }

    // Call: `Indicators.SimpleMovingAverage(Close, 20)` or `Math.Abs(x)` or `SMA(Close, 20)`
    if let Some(open) = cleaned.find('(') {
        if cleaned.ends_with(')') {
            let func_raw = cleaned[..open].trim();
            let func = func_raw
                .trim_start_matches("Indicators.")
                .trim_start_matches("Math.");
            let args_str = &cleaned[open + 1..cleaned.len() - 1];
            let parts = split_top_level_commas(args_str);
            let ir_args: Option<Vec<IrExpr>> =
                parts.iter().map(|a| parse_calgo_expr(a.trim())).collect();
            if let Some(ir_args) = ir_args {
                let mapped: Option<&str> = match func {
                    "SimpleMovingAverage" | "SMA" | "Sma" => Some("ta_sma"),
                    "ExponentialMovingAverage" | "EMA" | "Ema" => Some("ta_ema"),
                    "RelativeStrengthIndex" | "RSI" | "Rsi" => Some("ta_rsi"),
                    "AverageTrueRange" | "ATR" | "Atr" => Some("ta_atr"),
                    "Highest" => Some("ta_highest"),
                    "Lowest" => Some("ta_lowest"),
                    "StandardDeviation" | "StdDev" => Some("ta_stdev"),
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

    for op_str in &[
        " + ", " - ", " * ", " / ", " >= ", " <= ", " > ", " < ", " == ", " != ",
    ] {
        if let Some(pos) = cleaned.find(op_str) {
            let left = parse_calgo_expr(&cleaned[..pos])?;
            let right = parse_calgo_expr(&cleaned[pos + op_str.len()..])?;
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

    if cleaned.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Some(IrExpr::GetLocal(cleaned.to_ascii_lowercase()));
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

fn extract_first_quoted(s: &str) -> Option<String> {
    let start = s.find('"')?;
    let rest = &s[start + 1..];
    let end = rest.find('"')?;
    Some(rest[..end].to_string())
}

fn extract_named_arg(s: &str, name: &str) -> Option<String> {
    let pos = s.find(name)?;
    let rest = &s[pos + name.len()..];
    let eq = rest.find('=')?;
    let after = rest[eq + 1..].trim();
    // Stop at next `,` or `)` at top level
    let mut depth = 0i32;
    let mut end = after.len();
    for (i, c) in after.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                if depth == 0 {
                    end = i;
                    break;
                }
                depth -= 1;
            }
            ',' if depth == 0 => {
                end = i;
                break;
            }
            _ => {}
        }
    }
    Some(after[..end].trim().to_string())
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

fn strip_comments_and_regions(src: &str) -> String {
    let mut out = String::with_capacity(src.len());
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '/' && chars.peek() == Some(&'/') {
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
    // Drop #region / #endregion preprocessor lines
    out.lines()
        .filter(|l| {
            let t = l.trim();
            !t.starts_with("#region") && !t.starts_with("#endregion")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compile_calgo;

    #[test]
    fn test_calgo_simple_sma() {
        let src = r#"
[Indicator(IsOverlay = true, AccessRights = AccessRights.None)]
public class MySma : Indicator
{
    [Parameter("Period", DefaultValue = 20)]
    public int Period { get; set; }

    [Output("SMA", Color = Colors.Blue)]
    public IndicatorDataSeries Result { get; set; }

    public override void Calculate(int index)
    {
        Result[index] = Indicators.SimpleMovingAverage(Close, Period).Result[index];
    }
}
"#;
        let result = compile_calgo(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.short_name, "MySma");
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.inputs[0].name, "Period");
        assert_eq!(meta.inputs[0].default_value, "20");
        assert_eq!(meta.plots.len(), 1);
        assert_eq!(meta.plots[0].label, "SMA");
    }

    #[test]
    fn test_calgo_is_overlay_false() {
        let src = r#"
[Indicator(IsOverlay = false)]
public class X : Indicator {
    [Output("Line")]
    public IndicatorDataSeries Result { get; set; }
    public override void Calculate(int index) {
        Result[index] = Close[index];
    }
}
"#;
        let result = compile_calgo(src);
        assert!(result.metadata.unwrap().separate_window);
    }

    #[test]
    fn test_calgo_multi_output() {
        let src = r#"
public class D : Indicator {
    [Output("Fast")]
    public IndicatorDataSeries Fast { get; set; }
    [Output("Slow")]
    public IndicatorDataSeries Slow { get; set; }
    public override void Calculate(int index) {
        Fast[index] = Indicators.ExponentialMovingAverage(Close, 10).Result[index];
        Slow[index] = Indicators.ExponentialMovingAverage(Close, 30).Result[index];
    }
}
"#;
        let result = compile_calgo(src);
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 2);
        assert_eq!(meta.plots[0].label, "Fast");
        assert_eq!(meta.plots[1].label, "Slow");
    }

    #[test]
    fn test_calgo_math_abs() {
        if let Some(IrExpr::Call(name, _)) =
            parse_calgo_expr("Math.Abs(Close[index] - Open[index])")
        {
            assert_eq!(name, "math_abs");
        } else {
            panic!("Math.Abs should map to math_abs");
        }
    }

    #[test]
    fn test_calgo_sma_long_form() {
        if let Some(IrExpr::Call(name, _)) =
            parse_calgo_expr("Indicators.SimpleMovingAverage(Close, 20).Result[index]")
        {
            assert_eq!(name, "ta_sma");
        } else {
            panic!("SimpleMovingAverage.Result should map to ta_sma");
        }
    }

    #[test]
    fn test_calgo_bars_close_prices() {
        assert!(parse_calgo_expr("Bars.ClosePrices[index]").is_some());
        assert!(parse_calgo_expr("MarketSeries.Close[index]").is_some());
    }

    #[test]
    fn test_calgo_comment_stripping() {
        let src = r#"
// comment
/* block */
#region foo
public class X : Indicator {}
#endregion
"#;
        let result = compile_calgo(src);
        assert_eq!(result.metadata.unwrap().short_name, "X");
    }

    #[test]
    fn test_calgo_indicator_name_attribute() {
        let src = r#"
[Indicator(Name = "My Super Indicator", IsOverlay = true)]
public class C : Indicator {}
"#;
        let result = compile_calgo(src);
        assert_eq!(result.metadata.unwrap().short_name, "My Super Indicator");
    }

    #[test]
    fn test_calgo_parse_property() {
        let (n, t) = parse_calgo_property("public int Period { get; set; }").unwrap();
        assert_eq!(n, "Period");
        assert_eq!(t, "int");
    }

    #[test]
    fn test_calgo_extract_named_arg() {
        let s = r#""Period", DefaultValue = 14, MaxValue = 100"#;
        assert_eq!(extract_named_arg(s, "DefaultValue"), Some("14".into()));
    }

    #[test]
    fn test_calgo_empty_source() {
        let result = compile_calgo("");
        assert!(result.metadata.is_some());
    }
    #[test]
    fn test_calgo_duplicate_local_declared_once() {
        let src = r#"
[Indicator(Name = "Dup")]
public class Dup : Indicator {
    public override void Calculate(int index) {
        double value = Close[index];
        value = Open[index];
        Result[index] = value;
    }
}
"#;
        let (module, _) = build_ir(src);
        let locals = &module.on_calculate.as_ref().unwrap().locals;
        assert_eq!(locals.iter().filter(|(name, _)| name == "value").count(), 1);
    }
}
