//! WASM Code Generator — IR → WASM binary.
//!
//! Uses `wasm-encoder` to emit a valid WASM module that:
//! - Imports bar data functions (iOpen, iHigh, iLow, iClose, iVolume, iBars)
//! - Imports math functions (sqrt, log, abs, max, min)
//! - Exports `on_calculate(rates_total: i32, prev_calculated: i32) -> i32`
//! - Exports buffer memory for the runtime to read indicator values

use crate::ir::*;
use wasm_encoder::*;

/// Emit WASM binary from IR module.
pub fn emit_wasm(module: &IrModule) -> Result<Vec<u8>, String> {
    let mut wasm = Module::new();

    // ── Type section ─────────────────────────────────────────
    let mut types = TypeSection::new();

    // Type 0: () -> i32 (iBars)
    types.ty().function(vec![], vec![ValType::I32]);
    // Type 1: (i32) -> f64 (iOpen, iHigh, iLow, iClose, iVolume)
    types.ty().function(vec![ValType::I32], vec![ValType::F64]);
    // Type 2: (f64) -> f64 (math_abs, math_sqrt, math_log)
    types.ty().function(vec![ValType::F64], vec![ValType::F64]);
    // Type 3: (f64, f64) -> f64 (math_max, math_min)
    types
        .ty()
        .function(vec![ValType::F64, ValType::F64], vec![ValType::F64]);
    // Type 4: (i32, i32) -> i32 (on_calculate signature)
    types
        .ty()
        .function(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
    // Type 5: (i32, f64) -> () (set_buffer)
    types
        .ty()
        .function(vec![ValType::I32, ValType::F64], vec![]);

    wasm.section(&types);

    // ── Import section ───────────────────────────────────────
    let mut imports = ImportSection::new();

    // Runtime bar data imports
    imports.import("env", "iBars", EntityType::Function(0)); // 0
    imports.import("env", "iOpen", EntityType::Function(1)); // 1
    imports.import("env", "iHigh", EntityType::Function(1)); // 2
    imports.import("env", "iLow", EntityType::Function(1)); // 3
    imports.import("env", "iClose", EntityType::Function(1)); // 4
    imports.import("env", "iVolume", EntityType::Function(1)); // 5
    // Math imports
    imports.import("env", "math_abs", EntityType::Function(2)); // 6
    imports.import("env", "math_sqrt", EntityType::Function(2)); // 7
    imports.import("env", "math_log", EntityType::Function(2)); // 8
    imports.import("env", "math_max", EntityType::Function(3)); // 9
    imports.import("env", "math_min", EntityType::Function(3)); // 10
    // Buffer write import
    imports.import("env", "set_buffer", EntityType::Function(5)); // 11

    let num_imports = 12u32;
    wasm.section(&imports);

    // ── Function section ─────────────────────────────────────
    let mut functions = FunctionSection::new();
    // on_calculate function (type 4)
    functions.function(4);
    wasm.section(&functions);

    // ── Memory section ───────────────────────────────────────
    let mut memory = MemorySection::new();
    // 1 page = 64KB, enough for indicator buffers
    memory.memory(MemoryType {
        minimum: 1,
        maximum: Some(16),
        memory64: false,
        shared: false,
        page_size_log2: None,
    });
    wasm.section(&memory);

    // ── Export section ───────────────────────────────────────
    let mut exports = ExportSection::new();
    exports.export("on_calculate", ExportKind::Func, num_imports); // first local function
    exports.export("memory", ExportKind::Memory, 0);
    wasm.section(&exports);

    // ── Code section ─────────────────────────────────────────
    let mut code = CodeSection::new();

    // on_calculate function body. WebAssembly locals must be declared when the
    // function is constructed; keep the two codegen scratch locals first, then
    // append every typed local resolved by the IR lowering pass.
    let mut local_declarations = vec![
        // Local variables
        (1, ValType::I32), // i (loop counter)
        (1, ValType::F64), // temp f64
    ];
    if let Some(on_calc) = &module.on_calculate {
        local_declarations.extend(on_calc.locals.iter().map(|(_, ir_type)| {
            let value_type = match ir_type {
                IrType::I32 | IrType::Bool | IrType::String => ValType::I32,
                IrType::I64 => ValType::I64,
                IrType::F64 => ValType::F64,
            };
            (1, value_type)
        }));
    }
    let mut func = Function::new(local_declarations);

    if let Some(on_calc) = &module.on_calculate {
        // Emit the function body from IR
        emit_function_body(&mut func, on_calc, num_imports)?;
    } else {
        // Default: return prev_calculated
        func.instruction(&Instruction::LocalGet(1)); // prev_calculated
        func.instruction(&Instruction::End);
    }

    code.function(&func);
    wasm.section(&code);

    Ok(wasm.finish())
}

/// Local variable name→index mapping.
/// Params come first (0=rates_total, 1=prev_calculated), then declared locals.
#[allow(dead_code)]
struct LocalMap {
    map: std::collections::HashMap<String, u32>,
    next_idx: u32,
}

impl LocalMap {
    fn new(params: &[(String, IrType)], locals: &[(String, IrType)]) -> Self {
        let mut map = std::collections::HashMap::new();
        let mut idx = 0u32;
        for (name, _) in params {
            map.insert(name.clone(), idx);
            idx += 1;
        }
        // Fixed locals from Function::new (i: i32, temp: f64)
        let fixed_start = idx;
        idx = fixed_start + 2; // skip the 2 fixed locals
        for (name, _) in locals {
            map.insert(name.clone(), idx);
            idx += 1;
        }
        Self { map, next_idx: idx }
    }

    fn resolve(&self, name: &str) -> u32 {
        *self.map.get(name).unwrap_or(&2)
    }

    fn resolve_declared(&self, name: &str) -> Result<u32, String> {
        self.map
            .get(name)
            .copied()
            .ok_or_else(|| format!("unknown WASM local `{name}`"))
    }
}

fn emit_function_body(
    func: &mut Function,
    ir_func: &IrFunction,
    num_imports: u32,
) -> Result<(), String> {
    let local_map = LocalMap::new(&ir_func.params, &ir_func.locals);

    for stmt in &ir_func.body {
        emit_stmt_with_locals(func, stmt, num_imports, &local_map)?;
    }
    // Default return: rates_total (param 0)
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::End);
    Ok(())
}

#[allow(dead_code)] // used by tests
fn emit_stmt(func: &mut Function, stmt: &IrStmt, num_imports: u32) -> Result<(), String> {
    // Legacy wrapper for tests — uses default local index 2
    let default_map = LocalMap {
        map: std::collections::HashMap::new(),
        next_idx: 3,
    };
    emit_stmt_with_locals(func, stmt, num_imports, &default_map)
}

fn emit_stmt_with_locals(
    func: &mut Function,
    stmt: &IrStmt,
    num_imports: u32,
    locals: &LocalMap,
) -> Result<(), String> {
    match stmt {
        IrStmt::SetLocal(name, expr) => {
            emit_expr_with_locals(func, expr, num_imports, locals)?;
            func.instruction(&Instruction::LocalSet(locals.resolve_declared(name)?));
        }
        IrStmt::Return(Some(expr)) => {
            emit_expr_with_locals(func, expr, num_imports, locals)?;
            func.instruction(&Instruction::Return);
        }
        IrStmt::Return(None) => {
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::Return);
        }
        IrStmt::If { cond, then, else_ } => {
            emit_expr_with_locals(func, cond, num_imports, locals)?;
            func.instruction(&Instruction::If(BlockType::Empty));
            for s in then {
                emit_stmt_with_locals(func, s, num_imports, locals)?;
            }
            if !else_.is_empty() {
                func.instruction(&Instruction::Else);
                for s in else_ {
                    emit_stmt_with_locals(func, s, num_imports, locals)?;
                }
            }
            func.instruction(&Instruction::End);
        }
        IrStmt::Loop { body } => {
            func.instruction(&Instruction::Block(BlockType::Empty));
            func.instruction(&Instruction::Loop(BlockType::Empty));
            for s in body {
                emit_stmt_with_locals(func, s, num_imports, locals)?;
            }
            func.instruction(&Instruction::Br(0)); // continue loop
            func.instruction(&Instruction::End); // end loop
            func.instruction(&Instruction::End); // end block
        }
        IrStmt::Break => {
            func.instruction(&Instruction::Br(1)); // break out of block
        }
        IrStmt::Continue => {
            func.instruction(&Instruction::Br(0)); // branch to loop header
        }
        IrStmt::Expr(expr) => {
            emit_expr_with_locals(func, expr, num_imports, locals)?;
            func.instruction(&Instruction::Drop);
        }
        IrStmt::SetGlobal(_name, expr) => {
            emit_expr_with_locals(func, expr, num_imports, locals)?;
            // Global variables would need a global section — for now, store as local
            func.instruction(&Instruction::LocalSet(2));
        }
        IrStmt::SetBuffer(_buf_idx, bar_idx, value) => {
            emit_expr_with_locals(func, bar_idx, num_imports, locals)?;
            emit_expr_with_locals(func, value, num_imports, locals)?;
            func.instruction(&Instruction::Call(11)); // set_buffer import
        }
        IrStmt::Block(stmts) => {
            for s in stmts {
                emit_stmt_with_locals(func, s, num_imports, locals)?;
            }
        }
    }
    Ok(())
}

#[allow(dead_code)] // used by tests
fn emit_expr(func: &mut Function, expr: &IrExpr, num_imports: u32) -> Result<(), String> {
    let default_map = LocalMap {
        map: std::collections::HashMap::new(),
        next_idx: 3,
    };
    emit_expr_with_locals(func, expr, num_imports, &default_map)
}

fn emit_expr_with_locals(
    func: &mut Function,
    expr: &IrExpr,
    num_imports: u32,
    locals: &LocalMap,
) -> Result<(), String> {
    match expr {
        IrExpr::I32Const(n) => {
            func.instruction(&Instruction::I32Const(*n));
        }
        IrExpr::F64Const(f) => {
            func.instruction(&Instruction::F64Const((*f).into()));
        }
        IrExpr::GetLocal(name) => {
            func.instruction(&Instruction::LocalGet(locals.resolve(name)));
        }
        IrExpr::IBars => {
            func.instruction(&Instruction::Call(0)); // iBars import
        }
        IrExpr::IOpen(shift) => {
            emit_expr_with_locals(func, shift, num_imports, locals)?;
            func.instruction(&Instruction::Call(1)); // iOpen import
        }
        IrExpr::IHigh(shift) => {
            emit_expr_with_locals(func, shift, num_imports, locals)?;
            func.instruction(&Instruction::Call(2));
        }
        IrExpr::ILow(shift) => {
            emit_expr_with_locals(func, shift, num_imports, locals)?;
            func.instruction(&Instruction::Call(3));
        }
        IrExpr::IClose(shift) => {
            emit_expr_with_locals(func, shift, num_imports, locals)?;
            func.instruction(&Instruction::Call(4));
        }
        IrExpr::IVolume(shift) => {
            emit_expr_with_locals(func, shift, num_imports, locals)?;
            func.instruction(&Instruction::Call(5));
        }
        IrExpr::BinOp(op, left, right) => {
            emit_expr_with_locals(func, left, num_imports, locals)?;
            emit_expr_with_locals(func, right, num_imports, locals)?;
            match op {
                IrBinOp::AddF64 => {
                    func.instruction(&Instruction::F64Add);
                }
                IrBinOp::SubF64 => {
                    func.instruction(&Instruction::F64Sub);
                }
                IrBinOp::MulF64 => {
                    func.instruction(&Instruction::F64Mul);
                }
                IrBinOp::DivF64 => {
                    func.instruction(&Instruction::F64Div);
                }
                IrBinOp::AddI32 => {
                    func.instruction(&Instruction::I32Add);
                }
                IrBinOp::SubI32 => {
                    func.instruction(&Instruction::I32Sub);
                }
                IrBinOp::MulI32 => {
                    func.instruction(&Instruction::I32Mul);
                }
                IrBinOp::LtF64 => {
                    func.instruction(&Instruction::F64Lt);
                }
                IrBinOp::LeF64 => {
                    func.instruction(&Instruction::F64Le);
                }
                IrBinOp::GtF64 => {
                    func.instruction(&Instruction::F64Gt);
                }
                IrBinOp::GeF64 => {
                    func.instruction(&Instruction::F64Ge);
                }
                IrBinOp::EqF64 => {
                    func.instruction(&Instruction::F64Eq);
                }
                IrBinOp::NeF64 => {
                    func.instruction(&Instruction::F64Ne);
                }
                IrBinOp::And => {
                    func.instruction(&Instruction::I32And);
                }
                IrBinOp::Or => {
                    func.instruction(&Instruction::I32Or);
                }
                _ => {
                    func.instruction(&Instruction::F64Add);
                } // fallback
            }
        }
        IrExpr::UnaryOp(op, operand) => {
            emit_expr_with_locals(func, operand, num_imports, locals)?;
            match op {
                IrUnaryOp::NegF64 => {
                    func.instruction(&Instruction::F64Neg);
                }
                IrUnaryOp::NegI32 => {
                    func.instruction(&Instruction::I32Const(0));
                    func.instruction(&Instruction::I32Sub); // 0 - x
                }
                IrUnaryOp::Not => {
                    func.instruction(&Instruction::I32Eqz);
                }
            }
        }
        IrExpr::AssignLocal(name, value) => {
            emit_expr_with_locals(func, value, num_imports, locals)?;
            func.instruction(&Instruction::LocalTee(locals.resolve_declared(name)?));
        }
        IrExpr::Call(name, args) => {
            if name == "__select_f64" && args.len() == 3 {
                // WebAssembly `select` consumes operands as `then`, `else`, `cond`.
                // The IR stores ternary-like expressions as `cond`, `then`, `else`,
                // so emit this synthetic call in stack order instead of using the
                // normal left-to-right function-call argument order.
                emit_expr_with_locals(func, &args[1], num_imports, locals)?;
                emit_expr_with_locals(func, &args[2], num_imports, locals)?;
                emit_expr_with_locals(func, &args[0], num_imports, locals)?;
                func.instruction(&Instruction::Select);
                return Ok(());
            }

            for arg in args {
                emit_expr_with_locals(func, arg, num_imports, locals)?;
            }
            match name.as_str() {
                "math_abs" => {
                    func.instruction(&Instruction::Call(6));
                }
                "math_sqrt" => {
                    func.instruction(&Instruction::Call(7));
                }
                "math_log" => {
                    func.instruction(&Instruction::Call(8));
                }
                "math_max" => {
                    func.instruction(&Instruction::Call(9));
                }
                "math_min" => {
                    func.instruction(&Instruction::Call(10));
                }
                "set_buffer" => {
                    func.instruction(&Instruction::Call(11));
                }
                "__select_f64" if args.len() == 3 => {
                    // Ternary: cond, then, else already on stack
                    func.instruction(&Instruction::Select);
                }
                _ => {
                    // Unknown function — emit a NaN placeholder
                    tracing::warn!("Unknown MQL5 function: {}", name);
                    func.instruction(&Instruction::F64Const(f64::NAN.into()));
                }
            }
        }
        IrExpr::F64ToI32(inner) => {
            emit_expr_with_locals(func, inner, num_imports, locals)?;
            func.instruction(&Instruction::I32TruncF64S);
        }
        IrExpr::I32ToF64(inner) => {
            emit_expr_with_locals(func, inner, num_imports, locals)?;
            func.instruction(&Instruction::F64ConvertI32S);
        }
        _ => {
            func.instruction(&Instruction::F64Const(0.0f64.into()));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
