//! Cross-language indicator transpiler.
//!
//! Because every TyphooN frontend lowers into the same `IrModule`, adding a
//! new backend that emits source code in a target language lets us perform
//! **any language X → any language Y conversion** as `parse_X → IR → emit_Y`.
//!
//! This is an exclusive feature of the platform: traders can paste a
//! community indicator published in one language and get clean, runnable
//! code for their broker's native language with a single command.
//!
//! ## Supported directions (Phase 1 — this commit)
//!
//! | Source → Target     | MQL5 | Pine v5 | EasyLanguage | thinkScript |
//! |---------------------|:----:|:-------:|:------------:|:-----------:|
//! | EasyLanguage        |  ✅  |   ✅    |      —       |     ✅      |
//! | thinkScript         |  ✅  |   ✅    |     ✅       |      —      |
//! | AFL                 |  ✅  |   ✅    |     ✅       |     ✅      |
//! | ProBuilder          |  ✅  |   ✅    |     ✅       |     ✅      |
//! | Pine v4/v5          |  ✅  |    —    |     ✅       |     ✅      |
//!
//! Languages that require full C# / C parsers (NinjaScript, cAlgo, MQL4/5)
//! remain *sources only* in Phase 1 — IR → C# / MQL5 pretty-printers land
//! in Phase 2 (see ADR-090).
//!
//! ## Approach
//!
//! The line-scanner frontends all produce an IR `on_calculate` body made of
//! `SetLocal` / `SetBuffer` statements over a limited expression language
//! (`IClose/IOpen/…`, `BinOp`, `Call("ta_sma"|"ta_ema"|…)`, constants).
//! Each backend here translates that tree into valid target-language source
//! with a plot declaration for each buffer slot plus inputs mapped to the
//! target's idiomatic input syntax.

use crate::ir::*;
use crate::IndicatorMeta;

/// Set of languages the transpiler understands as a source.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLanguage {
    Mql5,
    Mql4,
    PineScript,
    EasyLanguage,
    ThinkScript,
    Afl,
    ProBuilder,
    NinjaScript,
    Calgo,
}

/// Set of languages the transpiler can emit. Phase 1 covers the four
/// heaviest community languages. Phase 2 will add the C#/C-family outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetLanguage {
    Mql5,
    PineScript,
    EasyLanguage,
    ThinkScript,
}

/// Top-level transpile entry: parse `source` as `from`, lower to IR,
/// then emit as `to`.
pub fn transpile(source: &str, from: SourceLanguage, to: TargetLanguage) -> Result<String, String> {
    let (ir, meta) = source_to_ir(source, from)?;
    Ok(emit(&ir, &meta, to))
}

/// Parse the source in the given language and return its IR + metadata.
pub fn source_to_ir(source: &str, lang: SourceLanguage) -> Result<(IrModule, IndicatorMeta), String> {
    match lang {
        SourceLanguage::EasyLanguage => Ok(crate::easylang::build_ir(source)),
        SourceLanguage::ThinkScript  => Ok(crate::thinkscript::build_ir(source)),
        SourceLanguage::PineScript   => Ok(crate::pine::build_ir(source)),
        SourceLanguage::Afl          => Ok(crate::afl::build_ir(source)),
        SourceLanguage::ProBuilder   => Ok(crate::probuilder::build_ir(source)),
        SourceLanguage::Mql5
        | SourceLanguage::Mql4
        | SourceLanguage::NinjaScript
        | SourceLanguage::Calgo => Err(format!(
            "{:?}: source-to-IR round-trip is a Phase 2 feature (see ADR-090). \
             Transpile from EasyLanguage / thinkScript / PineScript / AFL / ProBuilder \
             for now.",
            lang
        )),
    }
}

/// Emit the IR as source code in the target language.
pub fn emit(ir: &IrModule, meta: &IndicatorMeta, to: TargetLanguage) -> String {
    match to {
        TargetLanguage::Mql5         => emit_mql5(ir, meta),
        TargetLanguage::PineScript   => emit_pine_v5(ir, meta),
        TargetLanguage::EasyLanguage => emit_easylang(ir, meta),
        TargetLanguage::ThinkScript  => emit_thinkscript(ir, meta),
    }
}

// ── MQL5 backend ─────────────────────────────────────────────────────

fn emit_mql5(ir: &IrModule, meta: &IndicatorMeta) -> String {
    let mut out = String::new();
    // Header / properties
    let name = if meta.short_name.is_empty() { "Transpiled" } else { meta.short_name.as_str() };
    out.push_str(&format!("//+------------------------------------------------------------------+\n"));
    out.push_str(&format!("//|  {}  (transpiled by TyphooN Terminal)                            |\n", name));
    out.push_str(&format!("//+------------------------------------------------------------------+\n"));
    out.push_str(&format!("#property indicator_{}\n",
        if meta.separate_window { "separate_window" } else { "chart_window" }));
    out.push_str(&format!("#property indicator_shortname \"{}\"\n", name));
    out.push_str(&format!("#property indicator_buffers {}\n", meta.buffers.max(1)));
    out.push_str(&format!("#property indicator_plots   {}\n", meta.plots.len().max(1)));
    for (i, p) in meta.plots.iter().enumerate() {
        out.push_str(&format!("#property indicator_label{}  \"{}\"\n", i + 1, p.label));
        out.push_str(&format!("#property indicator_type{}   DRAW_LINE\n", i + 1));
        out.push_str(&format!("#property indicator_color{}  clrBlue\n", i + 1));
    }
    out.push('\n');
    // Inputs
    for inp in &ir.inputs {
        let (ty, default) = ir_input_default(inp);
        out.push_str(&format!("input {} {} = {};\n", ty, camel_case(&inp.name), default));
    }
    out.push('\n');
    // Buffer declarations
    for (i, _) in meta.plots.iter().enumerate() {
        out.push_str(&format!("double Buffer{}[];\n", i));
    }
    out.push('\n');
    // OnInit
    out.push_str("int OnInit() {\n");
    for (i, _) in meta.plots.iter().enumerate() {
        out.push_str(&format!("    SetIndexBuffer({}, Buffer{}, INDICATOR_DATA);\n", i, i));
    }
    out.push_str("    return INIT_SUCCEEDED;\n}\n\n");
    // OnCalculate
    out.push_str("int OnCalculate(const int rates_total, const int prev_calculated,\n");
    out.push_str("                const datetime &time[], const double &open[],\n");
    out.push_str("                const double &high[], const double &low[],\n");
    out.push_str("                const double &close[], const long &tick_volume[],\n");
    out.push_str("                const long &volume[], const int &spread[]) {\n");
    out.push_str("    int start = (prev_calculated > 0) ? prev_calculated - 1 : 0;\n");
    out.push_str("    for (int i = start; i < rates_total; i++) {\n");
    if let Some(ref f) = ir.on_calculate {
        // Declare locals once at the top of the for loop
        for (name, _ty) in &f.locals {
            out.push_str(&format!("        double {} = 0.0;\n", name));
        }
        for stmt in &f.body {
            out.push_str(&format!("        {}\n", emit_stmt_mql5(stmt, i_var("i"))));
        }
    }
    out.push_str("    }\n");
    out.push_str("    return rates_total;\n}\n");
    out
}

fn emit_stmt_mql5(s: &IrStmt, bar: &str) -> String {
    match s {
        IrStmt::SetLocal(name, e) => format!("{} = {};", name, emit_expr_mql5(e, bar)),
        IrStmt::SetBuffer(idx, _bar_idx, e) => {
            format!("Buffer{}[i] = {};", idx, emit_expr_mql5(e, bar))
        }
        _ => "// (unsupported stmt)".into(),
    }
}

fn emit_expr_mql5(e: &IrExpr, bar: &str) -> String {
    match e {
        IrExpr::I32Const(n) => format!("{}", n),
        IrExpr::F64Const(f) => format!("{}", f),
        IrExpr::GetLocal(n) => n.clone(),
        IrExpr::IOpen(_)    => format!("open[{}]", bar),
        IrExpr::IHigh(_)    => format!("high[{}]", bar),
        IrExpr::ILow(_)     => format!("low[{}]", bar),
        IrExpr::IClose(_)   => format!("close[{}]", bar),
        IrExpr::IVolume(_)  => format!("(double)tick_volume[{}]", bar),
        IrExpr::IBars       => "rates_total".into(),
        IrExpr::BinOp(op, l, r) => format!("({} {} {})",
            emit_expr_mql5(l, bar), binop_sym(op), emit_expr_mql5(r, bar)),
        IrExpr::Call(name, args) => {
            let a: Vec<String> = args.iter().map(|a| emit_expr_mql5(a, bar)).collect();
            mql5_builtin(name, &a, bar)
        }
        IrExpr::UnaryOp(_, inner) => format!("(-{})", emit_expr_mql5(inner, bar)),
        _ => "0.0".into(),
    }
}

fn mql5_builtin(name: &str, args: &[String], bar: &str) -> String {
    match name {
        "ta_sma"     => format!("iMA(_Symbol,_Period,(int)({}),0,MODE_SMA,PRICE_CLOSE)",
            args.get(1).cloned().unwrap_or_else(|| "14".into())),
        "ta_ema"     => format!("iMA(_Symbol,_Period,(int)({}),0,MODE_EMA,PRICE_CLOSE)",
            args.get(1).cloned().unwrap_or_else(|| "14".into())),
        "ta_rsi"     => format!("iRSI(_Symbol,_Period,(int)({}),PRICE_CLOSE)",
            args.get(1).or(args.first()).cloned().unwrap_or_else(|| "14".into())),
        "ta_atr"     => format!("iATR(_Symbol,_Period,(int)({}))",
            args.first().cloned().unwrap_or_else(|| "14".into())),
        "ta_highest" => format!("high[iHighest(_Symbol,_Period,MODE_HIGH,(int)({}),{})]",
            args.get(1).cloned().unwrap_or_else(|| "14".into()), bar),
        "ta_lowest"  => format!("low[iLowest(_Symbol,_Period,MODE_LOW,(int)({}),{})]",
            args.get(1).cloned().unwrap_or_else(|| "14".into()), bar),
        "ta_stdev"   => format!("iStdDev(_Symbol,_Period,(int)({}),0,MODE_SMA,PRICE_CLOSE)",
            args.get(1).cloned().unwrap_or_else(|| "20".into())),
        "math_abs"   => format!("MathAbs({})", args.first().cloned().unwrap_or_default()),
        "math_sqrt"  => format!("MathSqrt({})", args.first().cloned().unwrap_or_default()),
        "math_log"   => format!("MathLog({})", args.first().cloned().unwrap_or_default()),
        "math_max"   => format!("MathMax({},{})",
            args.first().cloned().unwrap_or_default(),
            args.get(1).cloned().unwrap_or_default()),
        "math_min"   => format!("MathMin({},{})",
            args.first().cloned().unwrap_or_default(),
            args.get(1).cloned().unwrap_or_default()),
        _ => format!("/* {} */ 0.0", name),
    }
}

// ── PineScript v5 backend ────────────────────────────────────────────

fn emit_pine_v5(ir: &IrModule, meta: &IndicatorMeta) -> String {
    let mut out = String::new();
    let name = if meta.short_name.is_empty() { "Transpiled" } else { meta.short_name.as_str() };
    out.push_str("//@version=5\n");
    out.push_str(&format!("indicator(\"{}\", overlay={})\n",
        name, !meta.separate_window));
    // Inputs
    for inp in &ir.inputs {
        let pine_ty = match inp.ir_type {
            IrType::I32 => "int",
            IrType::Bool => "bool",
            _ => "float",
        };
        let (_, default) = ir_input_default(inp);
        out.push_str(&format!("{} = input.{}({}, title=\"{}\")\n",
            snake_case(&inp.name), pine_ty, default, inp.name));
    }
    // Body: emit locals and plots
    if let Some(ref f) = ir.on_calculate {
        for stmt in &f.body {
            match stmt {
                IrStmt::SetLocal(name, e) => {
                    out.push_str(&format!("{} = {}\n", snake_case(name), emit_expr_pine(e)));
                }
                IrStmt::SetBuffer(idx, _, e) => {
                    let label = meta.plots.get(*idx)
                        .map(|p| p.label.clone())
                        .unwrap_or_else(|| format!("Plot{}", idx));
                    out.push_str(&format!("plot({}, title=\"{}\", color=color.blue)\n",
                        emit_expr_pine(e), label));
                }
                _ => {}
            }
        }
    }
    out
}

fn emit_expr_pine(e: &IrExpr) -> String {
    match e {
        IrExpr::I32Const(n) => format!("{}", n),
        IrExpr::F64Const(f) => format!("{}", f),
        IrExpr::GetLocal(n) => snake_case(n),
        IrExpr::IOpen(_)    => "open".into(),
        IrExpr::IHigh(_)    => "high".into(),
        IrExpr::ILow(_)     => "low".into(),
        IrExpr::IClose(_)   => "close".into(),
        IrExpr::IVolume(_)  => "volume".into(),
        IrExpr::IBars       => "bar_index".into(),
        IrExpr::BinOp(op, l, r) => format!("({} {} {})",
            emit_expr_pine(l), binop_sym(op), emit_expr_pine(r)),
        IrExpr::Call(name, args) => {
            let a: Vec<String> = args.iter().map(emit_expr_pine).collect();
            pine_builtin(name, &a)
        }
        IrExpr::UnaryOp(_, inner) => format!("(-{})", emit_expr_pine(inner)),
        _ => "0.0".into(),
    }
}

fn pine_builtin(name: &str, args: &[String]) -> String {
    let a = |i: usize| args.get(i).cloned().unwrap_or_else(|| "0".into());
    match name {
        "ta_sma"     => format!("ta.sma({}, {})", a(0), a(1)),
        "ta_ema"     => format!("ta.ema({}, {})", a(0), a(1)),
        "ta_rsi"     => format!("ta.rsi({}, {})", a(0), a(1)),
        "ta_atr"     => format!("ta.atr({})", a(0)),
        "ta_highest" => format!("ta.highest({}, {})", a(0), a(1)),
        "ta_lowest"  => format!("ta.lowest({}, {})", a(0), a(1)),
        "ta_stdev"   => format!("ta.stdev({}, {})", a(0), a(1)),
        "math_abs"   => format!("math.abs({})", a(0)),
        "math_sqrt"  => format!("math.sqrt({})", a(0)),
        "math_log"   => format!("math.log({})", a(0)),
        "math_max"   => format!("math.max({}, {})", a(0), a(1)),
        "math_min"   => format!("math.min({}, {})", a(0), a(1)),
        _            => format!("/* {} */ 0.0", name),
    }
}

// ── EasyLanguage backend ─────────────────────────────────────────────

fn emit_easylang(ir: &IrModule, meta: &IndicatorMeta) -> String {
    let mut out = String::new();
    // Inputs block
    if !ir.inputs.is_empty() {
        out.push_str("inputs:\n");
        let items: Vec<String> = ir.inputs.iter().map(|inp| {
            let (_, default) = ir_input_default(inp);
            format!("    {}({})", camel_case(&inp.name), default)
        }).collect();
        out.push_str(&items.join(",\n"));
        out.push_str(";\n\n");
    }
    // Locals — declare with 0 initialiser
    if let Some(ref f) = ir.on_calculate {
        if !f.locals.is_empty() {
            out.push_str("variables:\n");
            let items: Vec<String> = f.locals.iter().map(|(n, _)| {
                format!("    {}(0)", camel_case(n))
            }).collect();
            out.push_str(&items.join(",\n"));
            out.push_str(";\n\n");
        }
        for stmt in &f.body {
            match stmt {
                IrStmt::SetLocal(name, e) => {
                    out.push_str(&format!("{} = {};\n", camel_case(name), emit_expr_el(e)));
                }
                IrStmt::SetBuffer(idx, _, e) => {
                    let label = meta.plots.get(*idx)
                        .map(|p| p.label.clone())
                        .unwrap_or_else(|| format!("Plot{}", idx));
                    out.push_str(&format!("Plot{}({}, \"{}\");\n", idx + 1, emit_expr_el(e), label));
                }
                _ => {}
            }
        }
    }
    out
}

fn emit_expr_el(e: &IrExpr) -> String {
    match e {
        IrExpr::I32Const(n) => format!("{}", n),
        IrExpr::F64Const(f) => format!("{}", f),
        IrExpr::GetLocal(n) => camel_case(n),
        IrExpr::IOpen(_)    => "Open".into(),
        IrExpr::IHigh(_)    => "High".into(),
        IrExpr::ILow(_)     => "Low".into(),
        IrExpr::IClose(_)   => "Close".into(),
        IrExpr::IVolume(_)  => "Volume".into(),
        IrExpr::IBars       => "CurrentBar".into(),
        IrExpr::BinOp(op, l, r) => format!("({} {} {})",
            emit_expr_el(l), el_binop(op), emit_expr_el(r)),
        IrExpr::Call(name, args) => {
            let a: Vec<String> = args.iter().map(emit_expr_el).collect();
            el_builtin(name, &a)
        }
        IrExpr::UnaryOp(_, inner) => format!("(-{})", emit_expr_el(inner)),
        _ => "0".into(),
    }
}

fn el_binop(op: &IrBinOp) -> &'static str {
    match op {
        IrBinOp::EqF64 | IrBinOp::EqI32 => "=",
        IrBinOp::NeF64 | IrBinOp::NeI32 => "<>",
        _ => binop_sym(op),
    }
}

fn el_builtin(name: &str, args: &[String]) -> String {
    let a = |i: usize| args.get(i).cloned().unwrap_or_else(|| "0".into());
    match name {
        "ta_sma"     => format!("Average({}, {})", a(0), a(1)),
        "ta_ema"     => format!("XAverage({}, {})", a(0), a(1)),
        "ta_rsi"     => format!("RSI({}, {})", a(0), a(1)),
        "ta_atr"     => format!("ATR({})", a(0)),
        "ta_highest" => format!("Highest({}, {})", a(0), a(1)),
        "ta_lowest"  => format!("Lowest({}, {})", a(0), a(1)),
        "ta_stdev"   => format!("StdDev({}, {})", a(0), a(1)),
        "math_abs"   => format!("AbsValue({})", a(0)),
        "math_sqrt"  => format!("SquareRoot({})", a(0)),
        "math_log"   => format!("Log({})", a(0)),
        "math_max"   => format!("MaxList({}, {})", a(0), a(1)),
        "math_min"   => format!("MinList({}, {})", a(0), a(1)),
        _            => format!("{{ {} }} 0", name),
    }
}

// ── thinkScript backend ──────────────────────────────────────────────

fn emit_thinkscript(ir: &IrModule, meta: &IndicatorMeta) -> String {
    let mut out = String::new();
    if meta.separate_window { out.push_str("declare lower;\n"); }
    // Inputs
    for inp in &ir.inputs {
        let (_, default) = ir_input_default(inp);
        out.push_str(&format!("input {} = {};\n", snake_case(&inp.name), default));
    }
    if let Some(ref f) = ir.on_calculate {
        for stmt in &f.body {
            match stmt {
                IrStmt::SetLocal(name, e) => {
                    out.push_str(&format!("def {} = {};\n", snake_case(name), emit_expr_ts(e)));
                }
                IrStmt::SetBuffer(idx, _, e) => {
                    let label = meta.plots.get(*idx)
                        .map(|p| p.label.clone())
                        .unwrap_or_else(|| format!("Plot{}", idx));
                    out.push_str(&format!("plot {} = {};\n",
                        snake_case(&label), emit_expr_ts(e)));
                }
                _ => {}
            }
        }
    }
    out
}

fn emit_expr_ts(e: &IrExpr) -> String {
    match e {
        IrExpr::I32Const(n) => format!("{}", n),
        IrExpr::F64Const(f) => format!("{}", f),
        IrExpr::GetLocal(n) => snake_case(n),
        IrExpr::IOpen(_)    => "open".into(),
        IrExpr::IHigh(_)    => "high".into(),
        IrExpr::ILow(_)     => "low".into(),
        IrExpr::IClose(_)   => "close".into(),
        IrExpr::IVolume(_)  => "volume".into(),
        IrExpr::IBars       => "BarNumber()".into(),
        IrExpr::BinOp(op, l, r) => format!("({} {} {})",
            emit_expr_ts(l), binop_sym(op), emit_expr_ts(r)),
        IrExpr::Call(name, args) => {
            let a: Vec<String> = args.iter().map(emit_expr_ts).collect();
            ts_builtin(name, &a)
        }
        IrExpr::UnaryOp(_, inner) => format!("(-{})", emit_expr_ts(inner)),
        _ => "0".into(),
    }
}

fn ts_builtin(name: &str, args: &[String]) -> String {
    let a = |i: usize| args.get(i).cloned().unwrap_or_else(|| "0".into());
    match name {
        "ta_sma"     => format!("Average({}, {})", a(0), a(1)),
        "ta_ema"     => format!("ExpAverage({}, {})", a(0), a(1)),
        "ta_rsi"     => format!("RSI({}, {})", a(0), a(1)),
        "ta_atr"     => format!("ATR({})", a(0)),
        "ta_highest" => format!("Highest({}, {})", a(0), a(1)),
        "ta_lowest"  => format!("Lowest({}, {})", a(0), a(1)),
        "ta_stdev"   => format!("StDev({}, {})", a(0), a(1)),
        "math_abs"   => format!("AbsValue({})", a(0)),
        "math_sqrt"  => format!("Sqrt({})", a(0)),
        "math_log"   => format!("Log({})", a(0)),
        "math_max"   => format!("Max({}, {})", a(0), a(1)),
        "math_min"   => format!("Min({}, {})", a(0), a(1)),
        _            => format!("{{-- {} --}} 0", name),
    }
}

// ── Shared helpers ───────────────────────────────────────────────────

fn ir_input_default(inp: &IrInput) -> (&'static str, String) {
    match (&inp.ir_type, &inp.default) {
        (IrType::I32,  IrValue::I32(n))  => ("int",   n.to_string()),
        (IrType::I64,  IrValue::I64(n))  => ("long",  n.to_string()),
        (IrType::F64,  IrValue::F64(f))  => ("double", format!("{}", f)),
        (IrType::Bool, IrValue::Bool(b)) => ("bool",  b.to_string()),
        _ => ("double", "0.0".into()),
    }
}

fn binop_sym(op: &IrBinOp) -> &'static str {
    match op {
        IrBinOp::AddF64 | IrBinOp::AddI32 => "+",
        IrBinOp::SubF64 | IrBinOp::SubI32 => "-",
        IrBinOp::MulF64 | IrBinOp::MulI32 => "*",
        IrBinOp::DivF64 | IrBinOp::DivI32 => "/",
        IrBinOp::ModI32 => "%",
        IrBinOp::EqF64  | IrBinOp::EqI32 => "==",
        IrBinOp::NeF64  | IrBinOp::NeI32 => "!=",
        IrBinOp::LtF64  | IrBinOp::LtI32 => "<",
        IrBinOp::LeF64  | IrBinOp::LeI32 => "<=",
        IrBinOp::GtF64  | IrBinOp::GtI32 => ">",
        IrBinOp::GeF64  | IrBinOp::GeI32 => ">=",
        IrBinOp::And => "&&",
        IrBinOp::Or  => "||",
    }
}

fn i_var(s: &str) -> &str { s }

/// `length` → `Length`, `moving_avg` → `MovingAvg`.
fn camel_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut upper_next = true;
    for c in s.chars() {
        if c == '_' {
            upper_next = true;
        } else if upper_next {
            out.extend(c.to_uppercase());
            upper_next = false;
        } else {
            out.push(c);
        }
    }
    out
}

/// `Length` → `length`, `MovingAvg` → `movingavg` (thinkScript is case-sensitive
/// but uses lowercase convention for def/input names).
fn snake_case(s: &str) -> String {
    s.to_ascii_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn roundtrip(src: &str, from: SourceLanguage, to: TargetLanguage) -> String {
        transpile(src, from, to).expect("transpile should succeed")
    }

    #[test]
    fn el_to_mql5_simple_ema() {
        let src = r#"
inputs: Length(14);
variables: MA(0);
MA = XAverage(Close, Length);
Plot1(MA, "EMA");
"#;
        let out = roundtrip(src, SourceLanguage::EasyLanguage, TargetLanguage::Mql5);
        assert!(out.contains("input int Length = 14;"));
        assert!(out.contains("iMA(_Symbol,_Period"));
        assert!(out.contains("Buffer0[i]"));
        assert!(out.contains("MODE_EMA"));
        assert!(out.contains("#property indicator_shortname"));
    }

    #[test]
    fn el_to_pine_simple_sma() {
        let src = r#"
inputs: Length(20);
MA = Average(Close, Length);
Plot1(MA, "SMA");
"#;
        let out = roundtrip(src, SourceLanguage::EasyLanguage, TargetLanguage::PineScript);
        assert!(out.contains("//@version=5"));
        assert!(out.contains("indicator("));
        assert!(out.contains("ta.sma(close, length)"));
        assert!(out.contains("plot("));
        assert!(out.contains("title=\"SMA\""));
    }

    #[test]
    fn ts_to_el_roundtrip() {
        let src = r#"
input length = 14;
def ma = Average(close, length);
plot SMA = ma;
"#;
        let out = roundtrip(src, SourceLanguage::ThinkScript, TargetLanguage::EasyLanguage);
        assert!(out.contains("inputs:"));
        assert!(out.contains("Length(14)"));
        assert!(out.contains("Average(Close, Length)"));
        assert!(out.contains("Plot1"));
    }

    #[test]
    fn pine_to_thinkscript_rsi() {
        let src = r#"
//@version=5
indicator("RSI", overlay=false)
length = input.int(defval=14, title="Length")
r = ta.rsi(close, length)
plot(r, title="RSI", color=color.yellow)
"#;
        let out = roundtrip(src, SourceLanguage::PineScript, TargetLanguage::ThinkScript);
        assert!(out.contains("declare lower;"));
        assert!(out.contains("input length = 14;"));
        assert!(out.contains("RSI(close, length)"));
        assert!(out.contains("plot "));
    }

    #[test]
    fn afl_to_mql5_ema() {
        let src = r#"
_SECTION_BEGIN("Test");
ema20 = EMA(Close, 20);
Plot(ema20, "EMA20", colorBlue);
_SECTION_END();
"#;
        let out = roundtrip(src, SourceLanguage::Afl, TargetLanguage::Mql5);
        assert!(out.contains("Buffer0[i]"));
        assert!(out.contains("MODE_EMA"));
    }

    #[test]
    fn probuilder_to_easylang() {
        let src = r#"
ema20 = ExponentialAverage[20](close)
RETURN ema20 AS "EMA20"
"#;
        let out = roundtrip(src, SourceLanguage::ProBuilder, TargetLanguage::EasyLanguage);
        assert!(out.contains("XAverage"));
        assert!(out.contains("Plot1"));
    }

    #[test]
    fn unsupported_source_returns_error() {
        let result = transpile("", SourceLanguage::Mql5, TargetLanguage::PineScript);
        assert!(result.is_err());
        let err = result.err().unwrap();
        assert!(err.contains("Phase 2"));
    }

    #[test]
    fn pine_to_easylang_roundtrip() {
        let src = r#"
//@version=5
indicator("X", overlay=true)
length = input.int(defval=10, title="Length")
avg = ta.sma(close, length)
plot(avg, title="Avg")
"#;
        let out = roundtrip(src, SourceLanguage::PineScript, TargetLanguage::EasyLanguage);
        assert!(out.contains("inputs:"));
        assert!(out.contains("Average("));
    }

    #[test]
    fn camel_case_works() {
        assert_eq!(camel_case("length"), "Length");
        assert_eq!(camel_case("moving_avg"), "MovingAvg");
        assert_eq!(camel_case("fast_ema"), "FastEma");
    }

    #[test]
    fn el_to_mql5_math_abs() {
        let src = r#"
variables: diff(0);
diff = AbsValue(Close - Open);
Plot1(diff, "Diff");
"#;
        let out = roundtrip(src, SourceLanguage::EasyLanguage, TargetLanguage::Mql5);
        assert!(out.contains("MathAbs"));
    }
}
