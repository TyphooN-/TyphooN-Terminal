//! WGSL Code Generator — AST → WGSL compute shader string.
//!
//! Compiles MQL5 indicator AST directly to a WGSL compute shader that runs
//! one thread per bar. Bar data is read from a storage buffer with 5 f32 fields
//! per bar: [open, high, low, close, volume]. Indicator output is written to a
//! separate read_write storage buffer.
//!
//! Layout (per bar, stride = 5):
//!   bars[i * 5u + 0u] = open
//!   bars[i * 5u + 1u] = high
//!   bars[i * 5u + 2u] = low
//!   bars[i * 5u + 3u] = close
//!   bars[i * 5u + 4u] = volume

use crate::ast::*;
use crate::error::CompileError;
use crate::parser;
use std::collections::HashSet;

/// Bar data field offsets within the interleaved storage buffer.
const OPEN_OFFSET: u32 = 0;
const HIGH_OFFSET: u32 = 1;
const LOW_OFFSET: u32 = 2;
const CLOSE_OFFSET: u32 = 3;
const VOLUME_OFFSET: u32 = 4;
const BAR_STRIDE: u32 = 5;

/// Compile MQL5 source to a WGSL compute shader string.
pub fn compile_to_wgsl(source: &str) -> Result<String, CompileError> {
    let program = parser::parse_mql5(source)?;
    emit_wgsl(&program)
}

/// Emit WGSL compute shader from a parsed AST.
pub fn emit_wgsl(program: &Program) -> Result<String, CompileError> {
    let mut ctx = WgslCtx::new();

    // Collect inputs for the Params struct
    for item in &program.items {
        if let TopLevel::Input(input) = item {
            let wgsl_type = mql5_type_to_wgsl(&input.type_name);
            if wgsl_type != "/* string */" {
                ctx.add_param(input.name.clone(), wgsl_type.to_string());
            }
        }
    }

    // Find OnCalculate
    let on_calc = program.items.iter().find_map(|item| {
        if let TopLevel::Function(func) = item {
            if func.name == "OnCalculate" {
                return Some(func);
            }
        }
        None
    });

    let on_calc = on_calc
        .ok_or_else(|| CompileError::Internal("No OnCalculate function found".to_string()))?;

    // Collect helper functions (not OnCalculate, not OnInit)
    let helpers: Vec<&FunctionDef> = program
        .items
        .iter()
        .filter_map(|item| {
            if let TopLevel::Function(func) = item {
                if func.name != "OnCalculate" && func.name != "OnInit" {
                    return Some(func);
                }
            }
            None
        })
        .collect();

    // Build shader
    let mut out = String::new();

    // Bindings
    out.push_str("@group(0) @binding(0) var<storage, read> bars: array<f32>;\n");
    out.push_str("@group(0) @binding(1) var<storage, read_write> output: array<f32>;\n");
    out.push_str("@group(0) @binding(2) var<uniform> params: Params;\n\n");

    // Params struct
    out.push_str("struct Params {\n");
    // Always include bar_count
    out.push_str("    bar_count: u32,\n");
    for (name, ty) in &ctx.params {
        out.push_str(&format!("    {}: {},\n", name, ty));
    }
    out.push_str("}\n\n");

    // Helper functions
    for helper in &helpers {
        let ret_type = mql5_type_to_wgsl(&helper.return_type);
        let params_str: Vec<String> = helper
            .params
            .iter()
            .filter_map(|p| {
                let wt = mql5_type_to_wgsl(&p.type_name);
                if wt == "/* string */" {
                    return None;
                }
                Some(format!("{}: {}", p.name, wt))
            })
            .collect();
        out.push_str(&format!(
            "fn {}({}) -> {} {{\n",
            helper.name,
            params_str.join(", "),
            ret_type
        ));
        emit_stmts(&mut out, &helper.body, &mut ctx, 1)?;
        out.push_str("}\n\n");
    }

    // Main compute entry point
    out.push_str("@compute @workgroup_size(256)\n");
    out.push_str("fn main(@builtin(global_invocation_id) id: vec3<u32>) {\n");
    out.push_str("    let i = id.x;\n");
    out.push_str("    if (i >= params.bar_count) { return; }\n");

    // Emit OnCalculate body (skip params — we use `i` as the bar index)
    emit_stmts(&mut out, &on_calc.body, &mut ctx, 1)?;

    out.push_str("}\n");

    Ok(out)
}

/// Context for WGSL code generation.
struct WgslCtx {
    /// Input parameters for the Params struct. Vec preserves stable emission order.
    params: Vec<(String, String)>,
    /// O(1) input-parameter membership for expression emission.
    param_names: HashSet<String>,
    /// Variables already declared in the current scope (to avoid re-declaring).
    declared_vars: Vec<String>,
}

impl WgslCtx {
    fn new() -> Self {
        Self {
            params: Vec::new(),
            param_names: HashSet::new(),
            declared_vars: Vec::new(),
        }
    }

    fn add_param(&mut self, name: String, ty: String) {
        if self.param_names.insert(name.clone()) {
            self.params.push((name, ty));
        }
    }

    fn is_param(&self, name: &str) -> bool {
        self.param_names.contains(name)
    }
}

/// Map MQL5 type names to WGSL types.
fn mql5_type_to_wgsl(type_name: &str) -> &'static str {
    match type_name.trim() {
        "double" | "float" => "f32",
        "int" | "short" | "char" => "i32",
        "uint" | "ushort" | "uchar" | "color" => "u32",
        "long" | "ulong" | "datetime" => "i32", // WGSL has no i64, downcast
        "bool" => "bool",
        "string" => "/* string */",
        "void" => "f32", // void functions return f32 in compute context
        _ => "f32",
    }
}

/// Emit a list of statements at the given indentation level.
fn emit_stmts(
    out: &mut String,
    stmts: &[Stmt],
    ctx: &mut WgslCtx,
    indent: usize,
) -> Result<(), CompileError> {
    for stmt in stmts {
        emit_stmt(out, stmt, ctx, indent)?;
    }
    Ok(())
}

/// Indent helper.
fn ind(level: usize) -> String {
    "    ".repeat(level)
}

/// Emit a single statement.
fn emit_stmt(
    out: &mut String,
    stmt: &Stmt,
    ctx: &mut WgslCtx,
    indent: usize,
) -> Result<(), CompileError> {
    match stmt {
        Stmt::VarDecl(decl) => {
            let wgsl_type = mql5_type_to_wgsl(&decl.type_name);
            if wgsl_type == "/* string */" {
                // Skip string variables in WGSL
                return Ok(());
            }
            let prefix = ind(indent);
            if let Some(init) = &decl.init {
                let expr_str = emit_expr_str(init, ctx)?;
                if decl.is_const {
                    out.push_str(&format!("{}let {} = {};\n", prefix, decl.name, expr_str));
                } else {
                    out.push_str(&format!(
                        "{}var {}: {} = {};\n",
                        prefix, decl.name, wgsl_type, expr_str
                    ));
                }
            } else {
                let default = match wgsl_type {
                    "f32" => "0.0",
                    "i32" => "0i",
                    "u32" => "0u",
                    "bool" => "false",
                    _ => "0.0",
                };
                out.push_str(&format!(
                    "{}var {}: {} = {};\n",
                    prefix, decl.name, wgsl_type, default
                ));
            }
            ctx.declared_vars.push(decl.name.clone());
        }
        Stmt::Expr(expr) => {
            let prefix = ind(indent);
            // Handle assignments specially
            if let Expr::Assign { target, op, value } = expr {
                emit_assign(out, target, op, value, ctx, indent)?;
            } else if let Expr::PostIncr(inner) = expr {
                let inner_str = emit_expr_str(inner, ctx)?;
                out.push_str(&format!("{}{} = {} + 1i;\n", prefix, inner_str, inner_str));
            } else if let Expr::PostDecr(inner) = expr {
                let inner_str = emit_expr_str(inner, ctx)?;
                out.push_str(&format!("{}{} = {} - 1i;\n", prefix, inner_str, inner_str));
            } else {
                let expr_str = emit_expr_str(expr, ctx)?;
                out.push_str(&format!("{}{};\n", prefix, expr_str));
            }
        }
        Stmt::Return(expr) => {
            let prefix = ind(indent);
            if let Some(e) = expr {
                let expr_str = emit_expr_str(e, ctx)?;
                out.push_str(&format!("{}return {};\n", prefix, expr_str));
            } else {
                out.push_str(&format!("{}return;\n", prefix));
            }
        }
        Stmt::If {
            cond, then, else_, ..
        } => {
            let prefix = ind(indent);
            let cond_str = emit_expr_str(cond, ctx)?;
            out.push_str(&format!("{}if ({}) {{\n", prefix, cond_str));
            emit_stmts(out, then, ctx, indent + 1)?;
            if let Some(else_stmts) = else_ {
                out.push_str(&format!("{}}} else {{\n", prefix));
                emit_stmts(out, else_stmts, ctx, indent + 1)?;
            }
            out.push_str(&format!("{}}}\n", prefix));
        }
        Stmt::For {
            init,
            cond,
            step,
            body,
            ..
        } => {
            let prefix = ind(indent);
            // Emit init before loop
            if let Some(init_stmt) = init {
                emit_stmt(out, init_stmt, ctx, indent)?;
            }
            // Use WGSL loop { ... }
            out.push_str(&format!("{}loop {{\n", prefix));
            if let Some(cond_expr) = cond {
                let cond_str = emit_expr_str(cond_expr, ctx)?;
                out.push_str(&format!("{}    if (!({cond_str})) {{ break; }}\n", prefix));
            }
            emit_stmts(out, body, ctx, indent + 1)?;
            // Emit step (continuing block)
            if let Some(step_expr) = step {
                if let Expr::PostIncr(inner) = step_expr {
                    let inner_str = emit_expr_str(inner, ctx)?;
                    out.push_str(&format!(
                        "{}    {} = {} + 1i;\n",
                        prefix, inner_str, inner_str
                    ));
                } else if let Expr::PostDecr(inner) = step_expr {
                    let inner_str = emit_expr_str(inner, ctx)?;
                    out.push_str(&format!(
                        "{}    {} = {} - 1i;\n",
                        prefix, inner_str, inner_str
                    ));
                } else if let Expr::Assign { target, op, value } = step_expr {
                    emit_assign(out, target, op, value, ctx, indent + 1)?;
                } else {
                    let step_str = emit_expr_str(step_expr, ctx)?;
                    out.push_str(&format!("{}    {};\n", prefix, step_str));
                }
            }
            out.push_str(&format!("{}}}\n", prefix));
        }
        Stmt::While { cond, body, .. } => {
            let prefix = ind(indent);
            let cond_str = emit_expr_str(cond, ctx)?;
            out.push_str(&format!("{}loop {{\n", prefix));
            out.push_str(&format!("{}    if (!({cond_str})) {{ break; }}\n", prefix));
            emit_stmts(out, body, ctx, indent + 1)?;
            out.push_str(&format!("{}}}\n", prefix));
        }
        Stmt::DoWhile { body, cond, .. } => {
            let prefix = ind(indent);
            let cond_str = emit_expr_str(cond, ctx)?;
            out.push_str(&format!("{}loop {{\n", prefix));
            emit_stmts(out, body, ctx, indent + 1)?;
            out.push_str(&format!("{}    if (!({cond_str})) {{ break; }}\n", prefix));
            out.push_str(&format!("{}}}\n", prefix));
        }
        Stmt::Break => {
            out.push_str(&format!("{}break;\n", ind(indent)));
        }
        Stmt::Continue => {
            out.push_str(&format!("{}continue;\n", ind(indent)));
        }
        Stmt::Block(stmts) => {
            let prefix = ind(indent);
            out.push_str(&format!("{}{{\n", prefix));
            emit_stmts(out, stmts, ctx, indent + 1)?;
            out.push_str(&format!("{}}}\n", prefix));
        }
        Stmt::Switch {
            expr,
            cases,
            default,
            ..
        } => {
            let prefix = ind(indent);
            let expr_str = emit_expr_str(expr, ctx)?;
            out.push_str(&format!("{}switch {} {{\n", prefix, expr_str));
            for (val, stmts) in cases {
                let val_str = emit_expr_str(val, ctx)?;
                out.push_str(&format!("{}    case {}: {{\n", prefix, val_str));
                emit_stmts(out, stmts, ctx, indent + 2)?;
                out.push_str(&format!("{}    }}\n", prefix));
            }
            if let Some(default_stmts) = default {
                out.push_str(&format!("{}    default: {{\n", prefix));
                emit_stmts(out, default_stmts, ctx, indent + 2)?;
                out.push_str(&format!("{}    }}\n", prefix));
            }
            out.push_str(&format!("{}}}\n", prefix));
        }
        Stmt::Empty => {}
    }
    Ok(())
}

/// Emit an assignment statement.
fn emit_assign(
    out: &mut String,
    target: &Expr,
    op: &AssignOp,
    value: &Expr,
    ctx: &mut WgslCtx,
    indent: usize,
) -> Result<(), CompileError> {
    let prefix = ind(indent);

    // Check if target is an array index (buffer write)
    if let Expr::Index { array, index } = target {
        let array_str = emit_expr_str(array, ctx)?;
        let index_str = emit_expr_str(index, ctx)?;
        let value_str = emit_expr_str(value, ctx)?;

        // Indicator buffer writes → output array
        if is_buffer_name(&array_str) {
            let rhs = apply_assign_op(op, &format!("output[{}]", index_str), &value_str);
            out.push_str(&format!("{}output[{}] = {};\n", prefix, index_str, rhs));
            return Ok(());
        }
        let lhs = format!("{}[{}]", array_str, index_str);
        let rhs = apply_assign_op(op, &lhs, &value_str);
        out.push_str(&format!("{}{} = {};\n", prefix, lhs, rhs));
        return Ok(());
    }

    let target_str = emit_expr_str(target, ctx)?;
    let value_str = emit_expr_str(value, ctx)?;
    let rhs = apply_assign_op(op, &target_str, &value_str);
    out.push_str(&format!("{}{} = {};\n", prefix, target_str, rhs));
    Ok(())
}

/// Apply compound assignment operators.
fn apply_assign_op(op: &AssignOp, target: &str, value: &str) -> String {
    match op {
        AssignOp::Assign => value.to_string(),
        AssignOp::AddAssign => format!("{} + {}", target, value),
        AssignOp::SubAssign => format!("{} - {}", target, value),
        AssignOp::MulAssign => format!("{} * {}", target, value),
        AssignOp::DivAssign => format!("{} / {}", target, value),
        AssignOp::ModAssign => format!("{} % {}", target, value),
        AssignOp::AndAssign => format!("{} & {}", target, value),
        AssignOp::OrAssign => format!("{} | {}", target, value),
        AssignOp::XorAssign => format!("{} ^ {}", target, value),
        AssignOp::ShlAssign => format!("{} << {}", target, value),
        AssignOp::ShrAssign => format!("{} >> {}", target, value),
    }
}

/// Check if a name looks like an indicator buffer (ExtBuffer, Buffer, etc.).
fn is_buffer_name(name: &str) -> bool {
    name.starts_with("Ext") || name.ends_with("Buffer") || name.ends_with("buffer")
}

/// Emit an expression to a WGSL string.
fn emit_expr_str(expr: &Expr, ctx: &mut WgslCtx) -> Result<String, CompileError> {
    match expr {
        Expr::IntLit(n) => Ok(format!("{}i", n)),
        Expr::FloatLit(f) => {
            let s = format!("{}", f);
            // Ensure it has a decimal point for WGSL f32
            if s.contains('.') {
                Ok(s)
            } else {
                Ok(format!("{}.0", s))
            }
        }
        Expr::BoolLit(b) => Ok(format!("{}", b)),
        Expr::Null => Ok("0.0".to_string()),
        Expr::StringLit(_) => Ok("0.0".to_string()), // strings not supported in WGSL
        Expr::ColorLit(_) => Ok("0u".to_string()),
        Expr::Ident(name) => {
            // Map known MQL5 identifiers
            match name.as_str() {
                "EMPTY_VALUE" => Ok("0.0".to_string()),
                "NULL" | "INVALID_HANDLE" => Ok("0.0".to_string()),
                _ => {
                    // Check if it's a param
                    if ctx.is_param(name) {
                        Ok(format!("params.{}", name))
                    } else {
                        Ok(name.clone())
                    }
                }
            }
        }
        Expr::BinOp { op, left, right } => {
            let l = emit_expr_str(left, ctx)?;
            let r = emit_expr_str(right, ctx)?;
            let op_str = match op {
                BinOp::Add => "+",
                BinOp::Sub => "-",
                BinOp::Mul => "*",
                BinOp::Div => "/",
                BinOp::Mod => "%",
                BinOp::Eq => "==",
                BinOp::Ne => "!=",
                BinOp::Lt => "<",
                BinOp::Le => "<=",
                BinOp::Gt => ">",
                BinOp::Ge => ">=",
                BinOp::And => "&&",
                BinOp::Or => "||",
                BinOp::BitAnd => "&",
                BinOp::BitOr => "|",
                BinOp::BitXor => "^",
                BinOp::Shl => "<<",
                BinOp::Shr => ">>",
            };
            Ok(format!("({} {} {})", l, op_str, r))
        }
        Expr::UnaryOp { op, operand } => {
            let inner = emit_expr_str(operand, ctx)?;
            match op {
                UnaryOp::Neg => Ok(format!("(-{})", inner)),
                UnaryOp::Not => Ok(format!("!({})", inner)),
                UnaryOp::BitNot => Ok(format!("~({})", inner)),
                UnaryOp::PreIncr => Ok(format!("({} + 1i)", inner)),
                UnaryOp::PreDecr => Ok(format!("({} - 1i)", inner)),
            }
        }
        Expr::Call { func, args } => emit_call_str(func, args, ctx),
        Expr::Index { array, index } => {
            let array_str = emit_expr_str(array, ctx)?;
            let index_str = emit_expr_str(index, ctx)?;
            // Indicator buffer reads → output array
            if is_buffer_name(&array_str) {
                Ok(format!("output[{}]", index_str))
            } else {
                Ok(format!("{}[{}]", array_str, index_str))
            }
        }
        Expr::Member { object, field } => {
            let obj_str = emit_expr_str(object, ctx)?;
            Ok(format!("{}.{}", obj_str, field))
        }
        Expr::Ternary { cond, then, else_ } => {
            let c = emit_expr_str(cond, ctx)?;
            let t = emit_expr_str(then, ctx)?;
            let e = emit_expr_str(else_, ctx)?;
            Ok(format!("select({}, {}, {})", e, t, c))
        }
        Expr::Cast { target_type, expr } => {
            let inner = emit_expr_str(expr, ctx)?;
            let wgsl_type = mql5_type_to_wgsl(target_type);
            Ok(format!("{}({})", wgsl_type, inner))
        }
        Expr::PostIncr(inner) => {
            // In expression context, return current value (side effect handled at stmt level)
            emit_expr_str(inner, ctx)
        }
        Expr::PostDecr(inner) => emit_expr_str(inner, ctx),
        Expr::Assign { target, op, value } => {
            // Assignment as expression — emit the value
            let target_str = emit_expr_str(target, ctx)?;
            let value_str = emit_expr_str(value, ctx)?;
            let rhs = apply_assign_op(op, &target_str, &value_str);
            Ok(rhs)
        }
        Expr::ArrayInit(elems) => {
            let parts: Result<Vec<String>, _> =
                elems.iter().map(|e| emit_expr_str(e, ctx)).collect();
            Ok(format!("array({})", parts?.join(", ")))
        }
    }
}

/// Emit a function call, mapping MQL5 built-ins to WGSL equivalents.
fn emit_call_str(func: &str, args: &[Expr], ctx: &mut WgslCtx) -> Result<String, CompileError> {
    let arg_strs: Result<Vec<String>, _> = args.iter().map(|a| emit_expr_str(a, ctx)).collect();
    let arg_strs = arg_strs?;

    match func {
        // Bar data access — map to bar buffer reads
        // iOpen(symbol, timeframe, shift) → bars[shift * 5 + 0]
        "iOpen" => {
            let shift = bar_shift_arg(&arg_strs);
            Ok(format!(
                "bars[{} * {}u + {}u]",
                shift, BAR_STRIDE, OPEN_OFFSET
            ))
        }
        "iHigh" => {
            let shift = bar_shift_arg(&arg_strs);
            Ok(format!(
                "bars[{} * {}u + {}u]",
                shift, BAR_STRIDE, HIGH_OFFSET
            ))
        }
        "iLow" => {
            let shift = bar_shift_arg(&arg_strs);
            Ok(format!(
                "bars[{} * {}u + {}u]",
                shift, BAR_STRIDE, LOW_OFFSET
            ))
        }
        "iClose" => {
            let shift = bar_shift_arg(&arg_strs);
            Ok(format!(
                "bars[{} * {}u + {}u]",
                shift, BAR_STRIDE, CLOSE_OFFSET
            ))
        }
        "iVolume" => {
            let shift = bar_shift_arg(&arg_strs);
            Ok(format!(
                "bars[{} * {}u + {}u]",
                shift, BAR_STRIDE, VOLUME_OFFSET
            ))
        }
        "iBars" => Ok("params.bar_count".to_string()),

        // Math built-ins
        "MathSqrt" | "sqrt" => Ok(format!(
            "sqrt({})",
            arg_strs.first().unwrap_or(&"0.0".to_string())
        )),
        "MathAbs" | "fabs" => Ok(format!(
            "abs({})",
            arg_strs.first().unwrap_or(&"0.0".to_string())
        )),
        "MathPow" | "pow" => {
            let base = arg_strs.first().map(|s| s.as_str()).unwrap_or("0.0");
            let exp = arg_strs.get(1).map(|s| s.as_str()).unwrap_or("1.0");
            Ok(format!("pow({}, {})", base, exp))
        }
        "MathLog" | "log" => Ok(format!(
            "log({})",
            arg_strs.first().unwrap_or(&"0.0".to_string())
        )),
        "MathExp" | "exp" => Ok(format!(
            "exp({})",
            arg_strs.first().unwrap_or(&"0.0".to_string())
        )),
        "MathMax" | "fmax" => {
            let a = arg_strs.first().map(|s| s.as_str()).unwrap_or("0.0");
            let b = arg_strs.get(1).map(|s| s.as_str()).unwrap_or("0.0");
            Ok(format!("max({}, {})", a, b))
        }
        "MathMin" | "fmin" => {
            let a = arg_strs.first().map(|s| s.as_str()).unwrap_or("0.0");
            let b = arg_strs.get(1).map(|s| s.as_str()).unwrap_or("0.0");
            Ok(format!("min({}, {})", a, b))
        }
        "MathFloor" | "floor" => Ok(format!(
            "floor({})",
            arg_strs.first().unwrap_or(&"0.0".to_string())
        )),
        "MathCeil" | "ceil" => Ok(format!(
            "ceil({})",
            arg_strs.first().unwrap_or(&"0.0".to_string())
        )),
        "MathRound" | "round" => Ok(format!(
            "round({})",
            arg_strs.first().unwrap_or(&"0.0".to_string())
        )),
        "MathSin" | "sin" => Ok(format!(
            "sin({})",
            arg_strs.first().unwrap_or(&"0.0".to_string())
        )),
        "MathCos" | "cos" => Ok(format!(
            "cos({})",
            arg_strs.first().unwrap_or(&"0.0".to_string())
        )),
        "MathTan" | "tan" => Ok(format!(
            "tan({})",
            arg_strs.first().unwrap_or(&"0.0".to_string())
        )),
        "MathAtan" | "atan" => Ok(format!(
            "atan({})",
            arg_strs.first().unwrap_or(&"0.0".to_string())
        )),

        // Buffer operations — skip in WGSL (handled by runtime)
        "SetIndexBuffer"
        | "SetIndexStyle"
        | "IndicatorSetInteger"
        | "IndicatorSetString"
        | "IndicatorSetDouble"
        | "PlotIndexSetInteger"
        | "PlotIndexSetDouble"
        | "PlotIndexSetString"
        | "Print"
        | "Alert"
        | "Comment"
        | "PlaySound" => Ok("/* runtime-only */".to_string()),

        // ArraySetAsSeries and similar — no-op in compute shader
        "ArraySetAsSeries" | "ArrayResize" | "ArrayInitialize" | "ArraySize" | "ArrayCopy"
        | "ArrayFree" => Ok("/* array-op */".to_string()),

        // Default: emit as-is (user-defined function call)
        _ => Ok(format!("{}({})", func, arg_strs.join(", "))),
    }
}

/// Extract the bar shift argument from iOpen/iHigh/iLow/iClose/iVolume calls.
/// MQL5 signature: iClose(symbol, timeframe, shift) — we want the last arg.
/// For single-arg calls, use the only arg.
fn bar_shift_arg(args: &[String]) -> String {
    if args.len() >= 3 {
        // iClose(NULL, 0, i) → use the shift (3rd arg)
        args[2].clone()
    } else if args.len() == 1 {
        args[0].clone()
    } else {
        "i".to_string()
    }
}

#[cfg(test)]
mod tests;
