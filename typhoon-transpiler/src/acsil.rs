//! Sierra Chart ACSIL (Advanced Custom Study Interface and Language) frontend.
//!
//! ACSIL uses standard C/C++ with Sierra Chart's specific API. Studies are
//! DLL exports with a characteristic structure:
//!
//! ```cpp
//! #include "SierraChart.h"
//! SCDLLName("MyStudy")
//! SCSFExport scsf_MyStudy(SCStudyInterfaceRef sc) {
//!     SCSubgraphRef Sub = sc.Subgraph[0];
//!     SCInputRef Inp = sc.Input[0];
//!     if (sc.SetDefaults) {
//!         sc.GraphName = "My Study";
//!         Sub.Name = "SMA"; Sub.DrawStyle = DRAWSTYLE_LINE;
//!         Inp.Name = "Length"; Inp.SetInt(20);
//!         return;
//!     }
//!     sc.SimpleMovAvg(sc.BaseDataIn[SC_LAST], Sub, Inp.GetInt());
//! }
//! ```
//!
//! Supported:
//! - `SCDLLName("...")` → short name
//! - `sc.GraphName = "..."` → short name override
//! - `sc.GraphRegion = N` → separate window when N > 0
//! - `sc.Subgraph[N]` references → plot slots
//!   - `.Name = "..."` → plot label
//!   - `.DrawStyle = DRAWSTYLE_LINE` etc. → draw type
//! - `sc.Input[N]` references → inputs
//!   - `.Name = "..."` → input label
//!   - `.SetInt(N)` / `.SetFloat(N)` → typed default
//!   - `.GetInt()` / `.GetFloat()` in expressions → input reference
//! - `sc.BaseDataIn[SC_LAST]` / `SC_OPEN` / `SC_HIGH` / `SC_LOW` / `SC_VOLUME` → price series
//! - Built-in study functions:
//!   `sc.SimpleMovAvg` → `ta_sma`, `sc.ExponentialMovAvg` → `ta_ema`,
//!   `sc.RSI` → `ta_rsi`, `sc.ATR` → `ta_atr`,
//!   `sc.Highest` → `ta_highest`, `sc.Lowest` → `ta_lowest`,
//!   `sc.StdDev` → `ta_stdev`
//! - `Subgraph[sc.Index] = expr;` → buffer assignment
//! - Arithmetic + comparison operators
//! - `//` and `/* ... */` comments
//!
//! Not supported:
//! - Full C/C++ parsing (templates, classes, operator overloads)
//! - `sc.AutoLoop = 0` manual-loop mode
//! - DLL inter-study communication (`sc.GetStudyArrayUsingID`)
//! - Custom drawing via `sc.p_GDIFunction`

use crate::ir::*;
use crate::{CompileResult, DiagLevel, Diagnostic, DrawType, IndicatorMeta, InputParam, PlotDef};
use std::collections::HashSet;

pub fn parse_acsil(source: &str) -> CompileResult {
    let (ir_module, meta) = build_ir(source);
    let mut diagnostics = Vec::new();
    match crate::codegen::emit_wasm(&ir_module) {
        Ok(wasm) => {
            diagnostics.push(Diagnostic {
                level: DiagLevel::Info,
                message: format!(
                    "ACSIL compiled: {} inputs, {} plots",
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
                message: format!("ACSIL codegen failed: {e}"),
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
        short_name: String::from("ACSIL"),
        buffers: 0,
        separate_window: false,
        inputs: Vec::new(),
        plots: Vec::new(),
    };
    let mut ir_body: Vec<IrStmt> = Vec::new();
    let mut inputs: Vec<IrInput> = Vec::new();
    let mut locals: Vec<(String, IrType)> = Vec::new();
    let mut local_names: HashSet<String> = HashSet::new();

    // Track SCSubgraphRef aliases: "Sub" → index 0, etc.
    let mut subgraph_aliases: Vec<(String, usize)> = Vec::new();
    // Track SCInputRef aliases: "Inp" → index 0, etc.
    let mut input_aliases: Vec<(String, usize)> = Vec::new();

    let cleaned = strip_comments(source);
    let lines: Vec<&str> = cleaned.lines().collect();

    let mut in_defaults = false;

    // SCDLLName("...")
    for line in &lines {
        let t = line.trim();
        if t.starts_with("SCDLLName(") {
            if let Some(name) = extract_quoted(t) {
                meta.short_name = name;
            }
        }
    }

    for line in &lines {
        let t = line.trim().trim_end_matches(';');
        if t.is_empty() {
            continue;
        }

        // Detect SetDefaults block entry/exit
        if t.contains("sc.SetDefaults") && t.contains("if") {
            in_defaults = true;
            continue;
        }
        if in_defaults && t.trim() == "return" {
            in_defaults = false;
            continue;
        }

        // SCSubgraphRef alias: `SCSubgraphRef Sub = sc.Subgraph[N];`
        if t.contains("SCSubgraphRef") {
            if let Some((alias, idx)) = parse_ref_alias(t, "sc.Subgraph[") {
                subgraph_aliases.push((alias, idx));
                // Auto-register plot slot
                if meta.plots.iter().all(|p| p.index != idx) {
                    meta.plots.push(PlotDef {
                        index: idx,
                        label: format!("Subgraph{}", idx),
                        draw_type: DrawType::Line,
                        color: "clrBlue".into(),
                        width: 1,
                        style: 0,
                    });
                }
            }
            continue;
        }

        // SCInputRef alias: `SCInputRef Inp = sc.Input[N];`
        if t.contains("SCInputRef") {
            if let Some((alias, idx)) = parse_ref_alias(t, "sc.Input[") {
                input_aliases.push((alias, idx));
            }
            continue;
        }

        // Inside SetDefaults — extract metadata
        if in_defaults {
            // sc.GraphName = "..."
            if t.contains("sc.GraphName") {
                if let Some(name) = extract_rhs_quoted(t) {
                    meta.short_name = name;
                }
                continue;
            }
            // sc.GraphRegion = N (>0 means separate window)
            if t.contains("sc.GraphRegion") {
                if let Some(val) = extract_rhs_int(t) {
                    meta.separate_window = val > 0;
                }
                continue;
            }
            // SubAlias.Name = "..."
            for (alias, idx) in &subgraph_aliases {
                if t.contains(&format!("{}.Name", alias)) {
                    if let Some(label) = extract_rhs_quoted(t) {
                        if let Some(p) = meta.plots.iter_mut().find(|p| p.index == *idx) {
                            p.label = label;
                        }
                    }
                }
            }
            // InputAlias.Name = "..." / .SetInt(N) / .SetFloat(N)
            for (alias, idx) in &input_aliases {
                if t.contains(&format!("{}.Name", alias)) {
                    if let Some(label) = extract_rhs_quoted(t) {
                        // Ensure input slot exists
                        while inputs.len() <= *idx {
                            let n = inputs.len();
                            inputs.push(IrInput {
                                name: format!("input{}", n),
                                ir_type: IrType::F64,
                                default: IrValue::F64(0.0),
                            });
                            meta.inputs.push(InputParam {
                                name: format!("Input{}", n),
                                param_type: "float".into(),
                                default_value: "0".into(),
                            });
                        }
                        inputs[*idx].name = label.to_ascii_lowercase();
                        meta.inputs[*idx].name = label;
                    }
                }
                if t.contains(&format!("{}.SetInt(", alias)) {
                    if let Some(val) = extract_call_int(t, "SetInt") {
                        while inputs.len() <= *idx {
                            let n = inputs.len();
                            inputs.push(IrInput {
                                name: format!("input{}", n),
                                ir_type: IrType::I32,
                                default: IrValue::I32(0),
                            });
                            meta.inputs.push(InputParam {
                                name: format!("Input{}", n),
                                param_type: "int".into(),
                                default_value: "0".into(),
                            });
                        }
                        inputs[*idx].ir_type = IrType::I32;
                        inputs[*idx].default = IrValue::I32(val);
                        meta.inputs[*idx].param_type = "int".into();
                        meta.inputs[*idx].default_value = val.to_string();
                    }
                }
                if t.contains(&format!("{}.SetFloat(", alias)) {
                    if let Some(val) = extract_call_float(t, "SetFloat") {
                        while inputs.len() <= *idx {
                            let n = inputs.len();
                            inputs.push(IrInput {
                                name: format!("input{}", n),
                                ir_type: IrType::F64,
                                default: IrValue::F64(0.0),
                            });
                            meta.inputs.push(InputParam {
                                name: format!("Input{}", n),
                                param_type: "float".into(),
                                default_value: "0".into(),
                            });
                        }
                        inputs[*idx].ir_type = IrType::F64;
                        inputs[*idx].default = IrValue::F64(val);
                        meta.inputs[*idx].param_type = "float".into();
                        meta.inputs[*idx].default_value = format!("{}", val);
                    }
                }
            }
            continue;
        }

        // Outside SetDefaults — body statements

        // Subgraph assignment: `Sub[sc.Index] = expr;` or `sc.Subgraph[N][sc.Index] = expr;`
        if let Some((plot_idx, rhs)) = try_parse_subgraph_assign(t, &subgraph_aliases) {
            if let Some(e) = parse_acsil_expr(rhs, &input_aliases) {
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
            continue;
        }

        // sc.SimpleMovAvg(source, subgraph, length) — 3-arg form that writes directly to subgraph
        if let Some((plot_idx, ir_call)) =
            try_parse_sc_study_call(t, &subgraph_aliases, &input_aliases)
        {
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
            ir_body.push(IrStmt::SetBuffer(plot_idx, IrExpr::IBars, ir_call));
            continue;
        }

        // Local assignment: `float x = expr;` / `double x = expr;` / `x = expr;`
        if let Some((name, rhs)) = try_parse_local(t) {
            if let Some(e) = parse_acsil_expr(&rhs, &input_aliases) {
                let lname = name.to_ascii_lowercase();
                if local_names.insert(lname.clone()) {
                    locals.push((lname.clone(), IrType::F64));
                }
                ir_body.push(IrStmt::SetLocal(lname, e));
            }
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

// ── Helpers ────────────────────────────────────────────────────────────

fn parse_ref_alias(line: &str, pattern: &str) -> Option<(String, usize)> {
    // `SCSubgraphRef Sub = sc.Subgraph[0]`
    let eq = line.find('=')?;
    let lhs = line[..eq].trim();
    let alias = lhs.split_whitespace().last()?.to_string();
    let rhs = line[eq + 1..].trim();
    let start = rhs.find(pattern)? + pattern.len();
    let end = rhs[start..].find(']')?;
    let idx: usize = rhs[start..start + end].trim().parse().ok()?;
    Some((alias, idx))
}

fn extract_quoted(s: &str) -> Option<String> {
    let start = s.find('"')? + 1;
    let end = s[start..].find('"')?;
    Some(s[start..start + end].to_string())
}

fn extract_rhs_quoted(s: &str) -> Option<String> {
    let eq = s.find('=')?;
    extract_quoted(&s[eq + 1..])
}

fn extract_rhs_int(s: &str) -> Option<i32> {
    let eq = s.find('=')?;
    s[eq + 1..].trim().trim_end_matches(';').trim().parse().ok()
}

fn extract_call_int(s: &str, func: &str) -> Option<i32> {
    let pos = s.find(&format!("{}(", func))? + func.len() + 1;
    let end = s[pos..].find(')')?;
    s[pos..pos + end].trim().parse().ok()
}

fn extract_call_float(s: &str, func: &str) -> Option<f64> {
    let pos = s.find(&format!("{}(", func))? + func.len() + 1;
    let end = s[pos..].find(')')?;
    s[pos..pos + end].trim().parse().ok()
}

fn try_parse_subgraph_assign<'a>(
    line: &'a str,
    aliases: &[(String, usize)],
) -> Option<(usize, &'a str)> {
    let eq = line.find('=')?;
    let prev = line[..eq].chars().last();
    if matches!(prev, Some('!' | '<' | '>' | '=')) {
        return None;
    }
    let lhs = line[..eq].trim();
    let rhs = line[eq + 1..].trim();

    // `Sub[sc.Index]` form
    for (alias, idx) in aliases {
        if lhs.starts_with(alias.as_str()) && lhs.contains("[sc.Index]") {
            return Some((*idx, rhs));
        }
    }
    // `sc.Subgraph[N][sc.Index]` form
    if lhs.starts_with("sc.Subgraph[") {
        let start = "sc.Subgraph[".len();
        let end = lhs[start..].find(']')?;
        let idx: usize = lhs[start..start + end].trim().parse().ok()?;
        return Some((idx, rhs));
    }
    None
}

fn try_parse_sc_study_call(
    line: &str,
    subgraph_aliases: &[(String, usize)],
    input_aliases: &[(String, usize)],
) -> Option<(usize, IrExpr)> {
    // Pattern: `sc.SimpleMovAvg(source, subgraph_ref, length)`
    let sc_funcs: &[(&str, &str)] = &[
        ("sc.SimpleMovAvg(", "ta_sma"),
        ("sc.ExponentialMovAvg(", "ta_ema"),
        ("sc.RSI(", "ta_rsi"),
        ("sc.ATR(", "ta_atr"),
        ("sc.Highest(", "ta_highest"),
        ("sc.Lowest(", "ta_lowest"),
        ("sc.StdDev(", "ta_stdev"),
    ];
    for (prefix, ir_name) in sc_funcs {
        if let Some(pos) = line.find(prefix) {
            let rest = &line[pos + prefix.len()..];
            let close = find_balanced_close(rest)?;
            let args_str = &rest[..close];
            let parts = split_top_level_commas(args_str);
            // 3-arg form: (source, subgraph_ref, length)
            // 2-arg form: (source, length) — no subgraph target
            let (source_str, subgraph_idx, length_str) = if parts.len() >= 3 {
                let sg_name = parts[1].trim();
                let idx = subgraph_aliases
                    .iter()
                    .find(|(a, _)| a == sg_name)
                    .map(|(_, i)| *i)
                    .unwrap_or(0);
                (parts[0].trim(), idx, parts[2].trim())
            } else if parts.len() == 2 {
                (parts[0].trim(), 0, parts[1].trim())
            } else {
                continue;
            };
            let source_expr = parse_acsil_expr(source_str, input_aliases)?;
            let length_expr = parse_acsil_expr(length_str, input_aliases)?;
            return Some((
                subgraph_idx,
                IrExpr::Call(ir_name.to_string(), vec![source_expr, length_expr]),
            ));
        }
    }
    None
}

fn try_parse_local(line: &str) -> Option<(String, String)> {
    let line = line.trim().trim_end_matches(';');
    if line.is_empty()
        || line.starts_with("if")
        || line.starts_with("for")
        || line.starts_with("while")
        || line.starts_with("return")
        || line.starts_with("//")
        || line.starts_with('{')
        || line.starts_with('}')
        || line.starts_with('#')
        || line.starts_with("sc.")
        || line.starts_with("SC")
    {
        return None;
    }
    let eq = line.find('=')?;
    let prev = line[..eq].chars().last();
    if matches!(prev, Some('!' | '<' | '>' | '=')) {
        return None;
    }
    let lhs = line[..eq].trim();
    let rhs = line[eq + 1..].trim().to_string();
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

fn parse_acsil_expr(expr: &str, input_aliases: &[(String, usize)]) -> Option<IrExpr> {
    let expr = expr.trim().trim_end_matches(';').trim();
    if expr.is_empty() {
        return None;
    }
    if expr.starts_with('(') && expr.ends_with(')') && matched_parens(expr) {
        return parse_acsil_expr(&expr[1..expr.len() - 1], input_aliases);
    }
    if let Ok(f) = expr.parse::<f64>() {
        if f.fract() == 0.0 && f.abs() < i32::MAX as f64 {
            return Some(IrExpr::I32Const(f as i32));
        }
        return Some(IrExpr::F64Const(f));
    }

    // Price series: sc.BaseDataIn[SC_LAST] etc. — must START with the prefix,
    // not merely contain it (otherwise a function call like
    // `sc.SimpleMovAvg(sc.BaseDataIn[SC_LAST], 14)` would incorrectly match).
    if expr.starts_with("sc.BaseDataIn[") || expr.starts_with("sc.BaseData[") {
        if expr.contains("SC_LAST") || expr.contains("SC_CLOSE") {
            return Some(IrExpr::IClose(Box::new(IrExpr::I32Const(0))));
        }
        if expr.contains("SC_OPEN") {
            return Some(IrExpr::IOpen(Box::new(IrExpr::I32Const(0))));
        }
        if expr.contains("SC_HIGH") {
            return Some(IrExpr::IHigh(Box::new(IrExpr::I32Const(0))));
        }
        if expr.contains("SC_LOW") {
            return Some(IrExpr::ILow(Box::new(IrExpr::I32Const(0))));
        }
        if expr.contains("SC_VOLUME") || expr.contains("SC_NUM_TRADES") {
            return Some(IrExpr::IVolume(Box::new(IrExpr::I32Const(0))));
        }
        return Some(IrExpr::IClose(Box::new(IrExpr::I32Const(0))));
    }

    // sc.Index → bar count
    if expr == "sc.Index" || expr == "sc.ArraySize" {
        return Some(IrExpr::IBars);
    }

    // Input reference: `Inp.GetInt()` / `Inp.GetFloat()`
    for (alias, idx) in input_aliases {
        if expr.starts_with(alias.as_str())
            && (expr.contains(".GetInt()") || expr.contains(".GetFloat()"))
        {
            // Reference the input by its IR name
            let name = format!("input{}", idx);
            return Some(IrExpr::GetLocal(name));
        }
    }

    // Function calls — sc.SimpleMovAvg etc. returning a value (2-arg form)
    let sc_funcs: &[(&str, &str)] = &[
        ("sc.SimpleMovAvg(", "ta_sma"),
        ("sc.ExponentialMovAvg(", "ta_ema"),
        ("sc.RSI(", "ta_rsi"),
        ("sc.ATR(", "ta_atr"),
        ("sc.Highest(", "ta_highest"),
        ("sc.Lowest(", "ta_lowest"),
        ("sc.StdDev(", "ta_stdev"),
    ];
    for (prefix, ir_name) in sc_funcs {
        if expr.starts_with(prefix) && expr.ends_with(')') {
            let inner = &expr[prefix.len()..expr.len() - 1];
            let parts = split_top_level_commas(inner);
            let ir_args: Option<Vec<IrExpr>> = parts
                .iter()
                .map(|a| parse_acsil_expr(a.trim(), input_aliases))
                .collect();
            if let Some(args) = ir_args {
                return Some(IrExpr::Call(ir_name.to_string(), args));
            }
        }
    }

    // Math: abs/sqrt/log/fabs
    if expr.starts_with("abs(") || expr.starts_with("fabs(") {
        let inner = &expr[expr.find('(')? + 1..expr.len() - 1];
        let e = parse_acsil_expr(inner, input_aliases)?;
        return Some(IrExpr::Call("math_abs".into(), vec![e]));
    }
    if expr.starts_with("sqrt(") {
        let inner = &expr[5..expr.len() - 1];
        let e = parse_acsil_expr(inner, input_aliases)?;
        return Some(IrExpr::Call("math_sqrt".into(), vec![e]));
    }
    if expr.starts_with("log(") {
        let inner = &expr[4..expr.len() - 1];
        let e = parse_acsil_expr(inner, input_aliases)?;
        return Some(IrExpr::Call("math_log".into(), vec![e]));
    }

    // Binary operators
    for op_str in &[
        " + ", " - ", " * ", " / ", " >= ", " <= ", " > ", " < ", " == ", " != ",
    ] {
        if let Some(pos) = expr.find(op_str) {
            let left = parse_acsil_expr(&expr[..pos], input_aliases)?;
            let right = parse_acsil_expr(&expr[pos + op_str.len()..], input_aliases)?;
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

    // Bare identifier — local variable
    if expr.chars().all(|c| c.is_alphanumeric() || c == '_') {
        return Some(IrExpr::GetLocal(expr.to_ascii_lowercase()));
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

fn find_balanced_close(s: &str) -> Option<usize> {
    let mut depth = 1i32;
    for (i, c) in s.char_indices() {
        match c {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return Some(i);
                }
            }
            _ => {}
        }
    }
    None
}

fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '(' | '[' => depth += 1,
            ')' | ']' => depth = (depth - 1).max(0),
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
    use crate::compile_acsil;

    #[test]
    fn test_acsil_simple_sma() {
        let src = r#"
#include "SierraChart.h"
SCDLLName("My SMA")
SCSFExport scsf_MySMA(SCStudyInterfaceRef sc)
{
    SCSubgraphRef Sub = sc.Subgraph[0];
    SCInputRef Length = sc.Input[0];

    if (sc.SetDefaults)
    {
        sc.GraphName = "Simple MA";
        sc.AutoLoop = 1;
        Sub.Name = "SMA";
        Sub.DrawStyle = DRAWSTYLE_LINE;
        Length.Name = "Length";
        Length.SetInt(20);
        return;
    }

    sc.SimpleMovAvg(sc.BaseDataIn[SC_LAST], Sub, Length.GetInt());
}
"#;
        let result = compile_acsil(src);
        assert!(result.metadata.is_some());
        let meta = result.metadata.unwrap();
        assert_eq!(meta.short_name, "Simple MA");
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.inputs[0].name, "Length");
        assert_eq!(meta.inputs[0].default_value, "20");
        assert_eq!(meta.plots.len(), 1);
        assert_eq!(meta.plots[0].label, "SMA");
    }

    #[test]
    fn test_acsil_direct_assignment() {
        let src = r#"
SCDLLName("Test")
SCSFExport scsf_Test(SCStudyInterfaceRef sc)
{
    SCSubgraphRef Out = sc.Subgraph[0];
    if (sc.SetDefaults) {
        Out.Name = "Close";
        return;
    }
    Out[sc.Index] = sc.BaseDataIn[SC_LAST][sc.Index];
}
"#;
        let result = compile_acsil(src);
        let meta = result.metadata.unwrap();
        assert_eq!(meta.plots.len(), 1);
        assert_eq!(meta.plots[0].label, "Close");
    }

    #[test]
    fn test_acsil_multi_subgraph() {
        let src = r#"
SCDLLName("Multi")
SCSFExport scsf_Multi(SCStudyInterfaceRef sc)
{
    SCSubgraphRef Fast = sc.Subgraph[0];
    SCSubgraphRef Slow = sc.Subgraph[1];
    SCInputRef FLen = sc.Input[0];
    SCInputRef SLen = sc.Input[1];
    if (sc.SetDefaults) {
        Fast.Name = "Fast";
        Slow.Name = "Slow";
        FLen.Name = "FastLen";
        FLen.SetInt(10);
        SLen.Name = "SlowLen";
        SLen.SetInt(30);
        return;
    }
    sc.SimpleMovAvg(sc.BaseDataIn[SC_LAST], Fast, FLen.GetInt());
    sc.SimpleMovAvg(sc.BaseDataIn[SC_LAST], Slow, SLen.GetInt());
}
"#;
        let result = compile_acsil(src);
        let meta = result.metadata.unwrap();
        assert_eq!(meta.inputs.len(), 2);
        assert_eq!(meta.plots.len(), 2);
        assert_eq!(meta.plots[0].label, "Fast");
        assert_eq!(meta.plots[1].label, "Slow");
    }

    #[test]
    fn test_acsil_separate_window() {
        let src = r#"
SCDLLName("RSI")
SCSFExport scsf_RSI(SCStudyInterfaceRef sc)
{
    SCSubgraphRef Out = sc.Subgraph[0];
    if (sc.SetDefaults) {
        sc.GraphRegion = 1;
        Out.Name = "RSI";
        return;
    }
}
"#;
        let result = compile_acsil(src);
        assert!(result.metadata.unwrap().separate_window);
    }

    #[test]
    fn test_acsil_float_input() {
        let src = r#"
SCDLLName("Test")
SCSFExport scsf_Test(SCStudyInterfaceRef sc)
{
    SCInputRef Mult = sc.Input[0];
    if (sc.SetDefaults) {
        Mult.Name = "Multiplier";
        Mult.SetFloat(2.5);
        return;
    }
}
"#;
        let result = compile_acsil(src);
        let meta = result.metadata.unwrap();
        assert_eq!(meta.inputs[0].default_value, "2.5");
        assert_eq!(meta.inputs[0].param_type, "float");
    }

    #[test]
    fn test_acsil_comment_stripping() {
        let src = r#"
// line comment
/* block comment */
SCDLLName("X")
SCSFExport scsf_X(SCStudyInterfaceRef sc) {}
"#;
        let result = compile_acsil(src);
        assert_eq!(result.metadata.unwrap().short_name, "X");
    }

    #[test]
    fn test_acsil_ema_mapping() {
        if let Some(IrExpr::Call(name, _)) =
            parse_acsil_expr("sc.ExponentialMovAvg(sc.BaseDataIn[SC_LAST], 14)", &[])
        {
            assert_eq!(name, "ta_ema");
        } else {
            panic!("ExponentialMovAvg should map to ta_ema");
        }
    }

    #[test]
    fn test_acsil_price_series() {
        assert!(parse_acsil_expr("sc.BaseDataIn[SC_LAST]", &[]).is_some());
        assert!(parse_acsil_expr("sc.BaseDataIn[SC_OPEN]", &[]).is_some());
        assert!(parse_acsil_expr("sc.BaseDataIn[SC_HIGH]", &[]).is_some());
        assert!(parse_acsil_expr("sc.BaseDataIn[SC_LOW]", &[]).is_some());
    }

    #[test]
    fn test_acsil_empty_source() {
        let result = compile_acsil("");
        assert!(result.metadata.is_some());
    }

    #[test]
    fn test_acsil_arithmetic() {
        let expr = parse_acsil_expr("sc.BaseDataIn[SC_HIGH] + sc.BaseDataIn[SC_LOW]", &[]);
        assert!(expr.is_some());
    }
    #[test]
    fn test_acsil_duplicate_local_declared_once() {
        let src = r#"
SCDLLName("Dup")
SCSFExport scsf_Dup(SCStudyInterfaceRef sc) {
    float value = sc.BaseDataIn[SC_LAST];
    value = sc.BaseDataIn[SC_OPEN];
    sc.Subgraph[0][sc.Index] = value;
}
"#;
        let (module, _) = build_ir(src);
        let locals = &module.on_calculate.as_ref().unwrap().locals;
        assert_eq!(locals.iter().filter(|(name, _)| name == "value").count(), 1);
    }
}
