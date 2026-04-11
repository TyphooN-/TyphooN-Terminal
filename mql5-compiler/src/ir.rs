//! TyphooN Intermediate Representation (IR).
//!
//! Lowered from AST, ready for WASM code generation.
//! IR is typed, has resolved buffer indices, and explicit draw commands.

use crate::ast::*;
use crate::error::CompileError;
use crate::{IndicatorMeta, InputParam, DrawType};

/// IR module — ready for WASM codegen.
#[derive(Debug)]
pub struct IrModule {
    pub buffers: Vec<IrBuffer>,
    pub inputs: Vec<IrInput>,
    pub functions: Vec<IrFunction>,
    pub on_calculate: Option<IrFunction>,
    pub on_init: Option<IrFunction>,
    pub globals: Vec<IrGlobal>,
}

#[derive(Debug)]
pub struct IrBuffer {
    pub index: usize,
    pub draw_type: DrawType,
    pub color: String,
    pub width: u32,
    pub style: u32,
    pub label: String,
}

#[derive(Debug)]
pub struct IrInput {
    pub name: String,
    pub ir_type: IrType,
    pub default: IrValue,
}

#[derive(Debug, Clone)]
pub enum IrType {
    I32, I64, F64, Bool, String,
}

#[derive(Debug, Clone)]
pub enum IrValue {
    I32(i32), I64(i64), F64(f64), Bool(bool), String(String),
}

#[derive(Debug)]
pub struct IrGlobal {
    pub name: String,
    pub ir_type: IrType,
    pub init: Option<IrValue>,
}

#[derive(Debug)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<(String, IrType)>,
    pub return_type: IrType,
    pub body: Vec<IrStmt>,
    pub locals: Vec<(String, IrType)>,
}

#[derive(Debug)]
pub enum IrStmt {
    SetLocal(String, IrExpr),
    SetGlobal(String, IrExpr),
    SetBuffer(usize, IrExpr, IrExpr), // buffer_idx, bar_idx, value
    Return(Option<IrExpr>),
    If { cond: IrExpr, then: Vec<IrStmt>, else_: Vec<IrStmt> },
    Loop { body: Vec<IrStmt> },
    Break,
    Continue,
    Expr(IrExpr),
    Block(Vec<IrStmt>),
}

#[derive(Debug)]
pub enum IrExpr {
    I32Const(i32),
    F64Const(f64),
    GetLocal(String),
    GetGlobal(String),
    GetBuffer(usize, Box<IrExpr>), // buffer_idx, bar_idx
    // Bar data access (runtime imports)
    IOpen(Box<IrExpr>),   // shift
    IHigh(Box<IrExpr>),
    ILow(Box<IrExpr>),
    IClose(Box<IrExpr>),
    IVolume(Box<IrExpr>),
    IBars,
    // Math
    BinOp(IrBinOp, Box<IrExpr>, Box<IrExpr>),
    UnaryOp(IrUnaryOp, Box<IrExpr>),
    Call(String, Vec<IrExpr>),
    // Conversions
    F64ToI32(Box<IrExpr>),
    I32ToF64(Box<IrExpr>),
}

#[derive(Debug)]
pub enum IrBinOp {
    AddF64, SubF64, MulF64, DivF64,
    AddI32, SubI32, MulI32, DivI32, ModI32,
    EqF64, NeF64, LtF64, LeF64, GtF64, GeF64,
    EqI32, NeI32, LtI32, LeI32, GtI32, GeI32,
    And, Or,
}

#[derive(Debug)]
pub enum IrUnaryOp {
    NegF64, NegI32, Not,
}

/// Extract indicator metadata from AST (before full lowering).
pub fn extract_metadata(program: &Program) -> IndicatorMeta {
    let mut meta = IndicatorMeta {
        short_name: String::new(),
        buffers: 0,
        separate_window: false,
        inputs: Vec::new(),
        plots: Vec::new(),
    };

    for item in &program.items {
        match item {
            TopLevel::Property(prop) => {
                match prop.name.as_str() {
                    "indicator_shortname" => {
                        if let Expr::StringLit(s) = &prop.value {
                            meta.short_name = s.clone();
                        }
                    }
                    "indicator_separate_window" => {
                        meta.separate_window = true;
                    }
                    "indicator_buffers" => {
                        if let Expr::IntLit(n) = &prop.value {
                            meta.buffers = *n as usize;
                        }
                    }
                    _ => {}
                }
            }
            TopLevel::Input(input) => {
                meta.inputs.push(InputParam {
                    name: input.name.clone(),
                    param_type: input.type_name.clone(),
                    default_value: input.default.as_ref().map(|e| format!("{e:?}")).unwrap_or_default(),
                });
            }
            _ => {}
        }
    }

    meta
}

/// Lower AST to IR.
pub fn lower(program: &Program) -> Result<IrModule, Vec<CompileError>> {
    let mut module = IrModule {
        buffers: Vec::new(),
        inputs: Vec::new(),
        functions: Vec::new(),
        on_calculate: None,
        on_init: None,
        globals: Vec::new(),
    };

    let mut errors = Vec::new();

    for item in &program.items {
        match item {
            TopLevel::Input(input) => {
                let ir_type = mql5_type_to_ir(&input.type_name);
                let default = input.default.as_ref()
                    .map(|e| expr_to_ir_value(e))
                    .unwrap_or(IrValue::F64(0.0));
                module.inputs.push(IrInput {
                    name: input.name.clone(),
                    ir_type,
                    default,
                });
            }
            TopLevel::GlobalVar(var) => {
                let ir_type = mql5_type_to_ir(&var.type_name);
                let init = var.init.as_ref().map(|e| expr_to_ir_value(e));
                if var.is_array {
                    // Array buffers will be handled as WASM linear memory
                    module.globals.push(IrGlobal {
                        name: var.name.clone(),
                        ir_type,
                        init,
                    });
                } else {
                    module.globals.push(IrGlobal {
                        name: var.name.clone(),
                        ir_type,
                        init,
                    });
                }
            }
            TopLevel::Function(func) => {
                match func.name.as_str() {
                    "OnCalculate" => {
                        match lower_function(func) {
                            Ok(ir_func) => module.on_calculate = Some(ir_func),
                            Err(e) => errors.push(e),
                        }
                    }
                    "OnInit" => {
                        match lower_function(func) {
                            Ok(ir_func) => module.on_init = Some(ir_func),
                            Err(e) => errors.push(e),
                        }
                    }
                    _ => {
                        match lower_function(func) {
                            Ok(ir_func) => module.functions.push(ir_func),
                            Err(e) => errors.push(e),
                        }
                    }
                }
            }
            _ => {}
        }
    }

    if errors.is_empty() {
        Ok(module)
    } else {
        Err(errors)
    }
}

fn mql5_type_to_ir(type_name: &str) -> IrType {
    match type_name.trim() {
        "int" | "uint" | "short" | "ushort" | "char" | "uchar" | "bool" | "color" => IrType::I32,
        "long" | "ulong" | "datetime" => IrType::I64,
        "double" | "float" => IrType::F64,
        "string" => IrType::String,
        _ => IrType::F64, // default to f64 for unknown types
    }
}

fn expr_to_ir_value(expr: &Expr) -> IrValue {
    match expr {
        Expr::IntLit(n) => IrValue::I64(*n),
        Expr::FloatLit(f) => IrValue::F64(*f),
        Expr::BoolLit(b) => IrValue::Bool(*b),
        Expr::StringLit(s) => IrValue::String(s.clone()),
        _ => IrValue::F64(0.0),
    }
}

fn lower_function(func: &FunctionDef) -> Result<IrFunction, CompileError> {
    let return_type = mql5_type_to_ir(&func.return_type);
    let params: Vec<(String, IrType)> = func.params.iter()
        .map(|p| (p.name.clone(), mql5_type_to_ir(&p.type_name)))
        .collect();

    let mut locals = Vec::new();
    let body = lower_stmts(&func.body, &mut locals)?;

    Ok(IrFunction {
        name: func.name.clone(),
        params,
        return_type,
        body,
        locals,
    })
}

fn lower_stmts(stmts: &[Stmt], locals: &mut Vec<(String, IrType)>) -> Result<Vec<IrStmt>, CompileError> {
    let mut result = Vec::new();
    for stmt in stmts {
        result.push(lower_stmt(stmt, locals)?);
    }
    Ok(result)
}

fn lower_stmt(stmt: &Stmt, locals: &mut Vec<(String, IrType)>) -> Result<IrStmt, CompileError> {
    match stmt {
        Stmt::VarDecl(decl) => {
            let ir_type = mql5_type_to_ir(&decl.type_name);
            locals.push((decl.name.clone(), ir_type));
            if let Some(init) = &decl.init {
                Ok(IrStmt::SetLocal(decl.name.clone(), lower_expr(init)?))
            } else {
                Ok(IrStmt::SetLocal(decl.name.clone(), IrExpr::F64Const(0.0)))
            }
        }
        Stmt::Expr(expr) => Ok(IrStmt::Expr(lower_expr(expr)?)),
        Stmt::Return(expr) => {
            let ir_expr = expr.as_ref().map(|e| lower_expr(e)).transpose()?;
            Ok(IrStmt::Return(ir_expr))
        }
        Stmt::If { cond, then, else_, .. } => {
            let ir_cond = lower_expr(cond)?;
            let ir_then = lower_stmts(then, locals)?;
            let ir_else = else_.as_ref().map(|e| lower_stmts(e, locals)).transpose()?.unwrap_or_default();
            Ok(IrStmt::If { cond: ir_cond, then: ir_then, else_: ir_else })
        }
        Stmt::For { init, cond, step, body, .. } => {
            // Lower for-loop to a while-loop equivalent
            let mut loop_body = lower_stmts(body, locals)?;
            if let Some(step_expr) = step {
                loop_body.push(IrStmt::Expr(lower_expr(step_expr)?));
            }
            // Add condition check as break-if-not
            let loop_stmt = if let Some(cond_expr) = cond {
                let ir_cond = lower_expr(cond_expr)?;
                let break_check = IrStmt::If {
                    cond: IrExpr::UnaryOp(IrUnaryOp::Not, Box::new(ir_cond)),
                    then: vec![IrStmt::Break],
                    else_: vec![],
                };
                let mut full_body = vec![break_check];
                full_body.extend(loop_body);
                IrStmt::Loop { body: full_body }
            } else {
                IrStmt::Loop { body: loop_body }
            };

            let mut stmts = Vec::new();
            if let Some(init_stmt) = init {
                stmts.push(lower_stmt(init_stmt, locals)?);
            }
            stmts.push(loop_stmt);
            Ok(IrStmt::Block(stmts))
        }
        Stmt::While { cond, body, .. } => {
            let ir_cond = lower_expr(cond)?;
            let mut loop_body = vec![IrStmt::If {
                cond: IrExpr::UnaryOp(IrUnaryOp::Not, Box::new(ir_cond)),
                then: vec![IrStmt::Break],
                else_: vec![],
            }];
            loop_body.extend(lower_stmts(body, locals)?);
            Ok(IrStmt::Loop { body: loop_body })
        }
        Stmt::Break => Ok(IrStmt::Break),
        Stmt::Continue => Ok(IrStmt::Continue),
        Stmt::Block(stmts) => Ok(IrStmt::Block(lower_stmts(stmts, locals)?)),
        Stmt::DoWhile { cond, body, .. } => {
            // do { body } while(cond) → Loop { body; if(!cond) break; }
            let mut loop_body = lower_stmts(body, locals)?;
            let ir_cond = lower_expr(cond)?;
            loop_body.push(IrStmt::If {
                cond: IrExpr::UnaryOp(IrUnaryOp::Not, Box::new(ir_cond)),
                then: vec![IrStmt::Break],
                else_: vec![],
            });
            Ok(IrStmt::Loop { body: loop_body })
        }
        Stmt::Switch { expr, cases, default, .. } => {
            // Lower switch to chained if-else
            let switch_val = lower_expr(expr)?;
            let switch_local = format!("__switch_{}", locals.len());
            locals.push((switch_local.clone(), IrType::I32));
            let mut stmts = vec![IrStmt::SetLocal(switch_local.clone(), switch_val)];

            let _if_chain: Option<IrStmt> = None;
            // Build from last to first (nested if-else)
            let mut all_branches: Vec<(Expr, Vec<Stmt>)> = cases.clone();
            all_branches.reverse();

            let default_body = if let Some(def) = default {
                lower_stmts(def, locals)?
            } else {
                vec![]
            };
            let mut current_else = default_body;

            for (case_val, case_body) in &all_branches {
                let ir_case_val = lower_expr(case_val)?;
                let ir_body = lower_stmts(case_body, locals)?;
                let cond = IrExpr::BinOp(
                    IrBinOp::EqI32,
                    Box::new(IrExpr::GetLocal(switch_local.clone())),
                    Box::new(ir_case_val),
                );
                current_else = vec![IrStmt::If {
                    cond,
                    then: ir_body,
                    else_: current_else,
                }];
            }

            stmts.extend(current_else);
            Ok(IrStmt::Block(stmts))
        }
        Stmt::Empty => Ok(IrStmt::Expr(IrExpr::I32Const(0))),
    }
}

fn lower_expr(expr: &Expr) -> Result<IrExpr, CompileError> {
    match expr {
        Expr::IntLit(n) => Ok(IrExpr::I32Const(*n as i32)),
        Expr::FloatLit(f) => Ok(IrExpr::F64Const(*f)),
        Expr::BoolLit(b) => Ok(IrExpr::I32Const(if *b { 1 } else { 0 })),
        Expr::Null => Ok(IrExpr::F64Const(f64::NAN)),
        Expr::Ident(name) => Ok(IrExpr::GetLocal(name.clone())),
        Expr::BinOp { op, left, right } => {
            let l = lower_expr(left)?;
            let r = lower_expr(right)?;
            let ir_op = match op {
                BinOp::Add => IrBinOp::AddF64,
                BinOp::Sub => IrBinOp::SubF64,
                BinOp::Mul => IrBinOp::MulF64,
                BinOp::Div => IrBinOp::DivF64,
                BinOp::Eq => IrBinOp::EqF64,
                BinOp::Ne => IrBinOp::NeF64,
                BinOp::Lt => IrBinOp::LtF64,
                BinOp::Le => IrBinOp::LeF64,
                BinOp::Gt => IrBinOp::GtF64,
                BinOp::Ge => IrBinOp::GeF64,
                BinOp::And => IrBinOp::And,
                BinOp::Or => IrBinOp::Or,
                _ => IrBinOp::AddF64,
            };
            Ok(IrExpr::BinOp(ir_op, Box::new(l), Box::new(r)))
        }
        Expr::UnaryOp { op, operand } => {
            let inner = lower_expr(operand)?;
            let ir_op = match op {
                UnaryOp::Neg => IrUnaryOp::NegF64,
                UnaryOp::Not => IrUnaryOp::Not,
                _ => IrUnaryOp::NegF64,
            };
            Ok(IrExpr::UnaryOp(ir_op, Box::new(inner)))
        }
        Expr::Assign { target, value, .. } => {
            // Assignment is a statement-level operation but MQL5 allows it as an expression.
            // Lower to a Call that the codegen handles specially as set_local + get_local.
            let ir_value = lower_expr(value)?;
            match target.as_ref() {
                Expr::Ident(name) => {
                    // Emit as a special __assign call: codegen emits tee_local (set + return value)
                    Ok(IrExpr::Call("__assign".into(), vec![
                        IrExpr::GetLocal(name.clone()), // marker for which local
                        ir_value,
                    ]))
                }
                Expr::Index { array: _, index } => {
                    // Buffer assignment: array[index] = value
                    let ir_index = lower_expr(index)?;
                    // For now, buffer index 0 (will need proper resolution)
                    Ok(IrExpr::Call("set_buffer".into(), vec![ir_index, ir_value]))
                }
                _ => Ok(ir_value),
            }
        }
        Expr::Call { func, args } => {
            // Map MQL5 built-in functions to IR
            let ir_args: Vec<IrExpr> = args.iter().map(|a| lower_expr(a)).collect::<Result<_, _>>()?;
            match func.as_str() {
                "iOpen" if !ir_args.is_empty() => {
                    let last = ir_args.into_iter().last()
                        .ok_or_else(|| CompileError::Internal("expected arg after !is_empty guard".into()))?;
                    Ok(IrExpr::IOpen(Box::new(last)))
                }
                "iHigh" if !ir_args.is_empty() => {
                    let last = ir_args.into_iter().last()
                        .ok_or_else(|| CompileError::Internal("expected arg after !is_empty guard".into()))?;
                    Ok(IrExpr::IHigh(Box::new(last)))
                }
                "iLow" if !ir_args.is_empty() => {
                    let last = ir_args.into_iter().last()
                        .ok_or_else(|| CompileError::Internal("expected arg after !is_empty guard".into()))?;
                    Ok(IrExpr::ILow(Box::new(last)))
                }
                "iClose" if !ir_args.is_empty() => {
                    let last = ir_args.into_iter().last()
                        .ok_or_else(|| CompileError::Internal("expected arg after !is_empty guard".into()))?;
                    Ok(IrExpr::IClose(Box::new(last)))
                }
                "iVolume" if !ir_args.is_empty() => {
                    let last = ir_args.into_iter().last()
                        .ok_or_else(|| CompileError::Internal("expected arg after !is_empty guard".into()))?;
                    Ok(IrExpr::IVolume(Box::new(last)))
                }
                "iBars" => Ok(IrExpr::IBars),
                "MathMax" if ir_args.len() == 2 => {
                    let mut it = ir_args.into_iter();
                    let a = it.next().ok_or_else(|| CompileError::Internal("expected 2 args after len==2 guard".into()))?;
                    let b = it.next().ok_or_else(|| CompileError::Internal("expected 2 args after len==2 guard".into()))?;
                    Ok(IrExpr::Call("math_max".into(), vec![a, b]))
                }
                "MathMin" if ir_args.len() == 2 => {
                    let mut it = ir_args.into_iter();
                    let a = it.next().ok_or_else(|| CompileError::Internal("MathMin: expected 2 args".into()))?;
                    let b = it.next().ok_or_else(|| CompileError::Internal("MathMin: expected 2 args".into()))?;
                    Ok(IrExpr::Call("math_min".into(), vec![a, b]))
                }
                "MathAbs" if ir_args.len() == 1 => {
                    Ok(IrExpr::Call("math_abs".into(), ir_args))
                }
                "MathSqrt" if ir_args.len() == 1 => {
                    Ok(IrExpr::Call("math_sqrt".into(), ir_args))
                }
                "MathLog" if ir_args.len() == 1 => {
                    Ok(IrExpr::Call("math_log".into(), ir_args))
                }
                _ => Ok(IrExpr::Call(func.clone(), ir_args)),
            }
        }
        Expr::Index { array, index } => {
            let _ir_array = lower_expr(array)?;
            let ir_index = lower_expr(index)?;
            // For now, treat as buffer access
            Ok(IrExpr::GetBuffer(0, Box::new(ir_index)))
        }
        Expr::Ternary { cond, then, else_ } => {
            // Lower ternary to a call (WASM select)
            let c = lower_expr(cond)?;
            let t = lower_expr(then)?;
            let e = lower_expr(else_)?;
            Ok(IrExpr::Call("__select_f64".into(), vec![c, t, e]))
        }
        _ => Ok(IrExpr::F64Const(0.0)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ir_module_empty() {
        let m = IrModule {
            buffers: vec![], inputs: vec![], functions: vec![],
            on_calculate: None, on_init: None, globals: vec![],
        };
        assert!(m.buffers.is_empty());
        assert!(m.on_calculate.is_none());
    }

    #[test]
    fn ir_type_variants() {
        let types = [IrType::I32, IrType::I64, IrType::F64, IrType::Bool, IrType::String];
        assert_eq!(types.len(), 5);
    }

    #[test]
    fn extract_metadata_empty_program() {
        let p = Program { items: vec![] };
        let meta = extract_metadata(&p);
        assert_eq!(meta.buffers, 0);
        assert!(!meta.separate_window);
        assert!(meta.inputs.is_empty());
    }

    #[test]
    fn extract_metadata_with_properties() {
        let p = Program {
            items: vec![
                TopLevel::Property(Property { name: "indicator_separate_window".into(), value: Expr::IntLit(1), line: 1 }),
                TopLevel::Property(Property { name: "indicator_buffers".into(), value: Expr::IntLit(3), line: 2 }),
                TopLevel::Property(Property { name: "indicator_shortname".into(), value: Expr::StringLit("Test".into()), line: 3 }),
            ],
        };
        let meta = extract_metadata(&p);
        assert!(meta.separate_window);
        assert_eq!(meta.buffers, 3);
        assert_eq!(meta.short_name, "Test");
    }

    #[test]
    fn extract_metadata_with_inputs() {
        let p = Program {
            items: vec![
                TopLevel::Input(InputDecl { type_name: "int".into(), name: "Period".into(), default: Some(Expr::IntLit(14)), line: 1 }),
            ],
        };
        let meta = extract_metadata(&p);
        assert_eq!(meta.inputs.len(), 1);
        assert_eq!(meta.inputs[0].name, "Period");
    }

    #[test]
    fn lower_empty_program() {
        let p = Program { items: vec![] };
        let result = lower(&p);
        assert!(result.is_ok());
        let m = result.unwrap();
        assert!(m.on_calculate.is_none());
    }
}
