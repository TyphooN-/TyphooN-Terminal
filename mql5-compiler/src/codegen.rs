//! WASM Code Generator — IR → WASM binary.
//!
//! Uses `wasm-encoder` to emit a valid WASM module that:
//! - Imports bar data functions (iOpen, iHigh, iLow, iClose, iVolume, iBars)
//! - Imports math functions (sqrt, log, abs, max, min)
//! - Exports `on_calculate(rates_total: i32, prev_calculated: i32) -> i32`
//! - Exports buffer memory for the runtime to read indicator values

use wasm_encoder::*;
use crate::ir::*;

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
    types.ty().function(vec![ValType::F64, ValType::F64], vec![ValType::F64]);
    // Type 4: (i32, i32) -> i32 (on_calculate signature)
    types.ty().function(vec![ValType::I32, ValType::I32], vec![ValType::I32]);
    // Type 5: (i32, f64) -> () (set_buffer)
    types.ty().function(vec![ValType::I32, ValType::F64], vec![]);

    wasm.section(&types);

    // ── Import section ───────────────────────────────────────
    let mut imports = ImportSection::new();

    // Runtime bar data imports
    imports.import("env", "iBars", EntityType::Function(0));      // 0
    imports.import("env", "iOpen", EntityType::Function(1));      // 1
    imports.import("env", "iHigh", EntityType::Function(1));      // 2
    imports.import("env", "iLow", EntityType::Function(1));       // 3
    imports.import("env", "iClose", EntityType::Function(1));     // 4
    imports.import("env", "iVolume", EntityType::Function(1));    // 5
    // Math imports
    imports.import("env", "math_abs", EntityType::Function(2));   // 6
    imports.import("env", "math_sqrt", EntityType::Function(2));  // 7
    imports.import("env", "math_log", EntityType::Function(2));   // 8
    imports.import("env", "math_max", EntityType::Function(3));   // 9
    imports.import("env", "math_min", EntityType::Function(3));   // 10
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

    // on_calculate function body
    let mut func = Function::new(vec![
        // Local variables
        (1, ValType::I32), // i (loop counter)
        (1, ValType::F64), // temp f64
    ]);

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

fn emit_function_body(func: &mut Function, ir_func: &IrFunction, num_imports: u32) -> Result<(), String> {
    for stmt in &ir_func.body {
        emit_stmt(func, stmt, num_imports)?;
    }
    // Default return: rates_total (param 0)
    func.instruction(&Instruction::LocalGet(0));
    func.instruction(&Instruction::End);
    Ok(())
}

fn emit_stmt(func: &mut Function, stmt: &IrStmt, num_imports: u32) -> Result<(), String> {
    match stmt {
        IrStmt::SetLocal(_name, expr) => {
            emit_expr(func, expr, num_imports)?;
            // TODO: resolve local index from name
            func.instruction(&Instruction::LocalSet(2)); // placeholder
        }
        IrStmt::Return(Some(expr)) => {
            emit_expr(func, expr, num_imports)?;
            func.instruction(&Instruction::Return);
        }
        IrStmt::Return(None) => {
            func.instruction(&Instruction::I32Const(0));
            func.instruction(&Instruction::Return);
        }
        IrStmt::If { cond, then, else_ } => {
            emit_expr(func, cond, num_imports)?;
            func.instruction(&Instruction::If(BlockType::Empty));
            for s in then { emit_stmt(func, s, num_imports)?; }
            if !else_.is_empty() {
                func.instruction(&Instruction::Else);
                for s in else_ { emit_stmt(func, s, num_imports)?; }
            }
            func.instruction(&Instruction::End);
        }
        IrStmt::Loop { body } => {
            func.instruction(&Instruction::Block(BlockType::Empty));
            func.instruction(&Instruction::Loop(BlockType::Empty));
            for s in body { emit_stmt(func, s, num_imports)?; }
            func.instruction(&Instruction::Br(0)); // continue loop
            func.instruction(&Instruction::End); // end loop
            func.instruction(&Instruction::End); // end block
        }
        IrStmt::Break => {
            func.instruction(&Instruction::Br(1)); // break out of block
        }
        IrStmt::Expr(expr) => {
            emit_expr(func, expr, num_imports)?;
            func.instruction(&Instruction::Drop);
        }
        IrStmt::Block(stmts) => {
            for s in stmts { emit_stmt(func, s, num_imports)?; }
        }
        _ => {}
    }
    Ok(())
}

fn emit_expr(func: &mut Function, expr: &IrExpr, num_imports: u32) -> Result<(), String> {
    match expr {
        IrExpr::I32Const(n) => { func.instruction(&Instruction::I32Const(*n)); }
        IrExpr::F64Const(f) => { func.instruction(&Instruction::F64Const((*f).into())); }
        IrExpr::GetLocal(_name) => {
            // TODO: resolve local index
            func.instruction(&Instruction::LocalGet(2));
        }
        IrExpr::IBars => {
            func.instruction(&Instruction::Call(0)); // iBars import
        }
        IrExpr::IOpen(shift) => {
            emit_expr(func, shift, num_imports)?;
            func.instruction(&Instruction::Call(1)); // iOpen import
        }
        IrExpr::IHigh(shift) => {
            emit_expr(func, shift, num_imports)?;
            func.instruction(&Instruction::Call(2));
        }
        IrExpr::ILow(shift) => {
            emit_expr(func, shift, num_imports)?;
            func.instruction(&Instruction::Call(3));
        }
        IrExpr::IClose(shift) => {
            emit_expr(func, shift, num_imports)?;
            func.instruction(&Instruction::Call(4));
        }
        IrExpr::IVolume(shift) => {
            emit_expr(func, shift, num_imports)?;
            func.instruction(&Instruction::Call(5));
        }
        IrExpr::BinOp(op, left, right) => {
            emit_expr(func, left, num_imports)?;
            emit_expr(func, right, num_imports)?;
            match op {
                IrBinOp::AddF64 => { func.instruction(&Instruction::F64Add); }
                IrBinOp::SubF64 => { func.instruction(&Instruction::F64Sub); }
                IrBinOp::MulF64 => { func.instruction(&Instruction::F64Mul); }
                IrBinOp::DivF64 => { func.instruction(&Instruction::F64Div); }
                IrBinOp::AddI32 => { func.instruction(&Instruction::I32Add); }
                IrBinOp::SubI32 => { func.instruction(&Instruction::I32Sub); }
                IrBinOp::MulI32 => { func.instruction(&Instruction::I32Mul); }
                IrBinOp::LtF64 => { func.instruction(&Instruction::F64Lt); }
                IrBinOp::LeF64 => { func.instruction(&Instruction::F64Le); }
                IrBinOp::GtF64 => { func.instruction(&Instruction::F64Gt); }
                IrBinOp::GeF64 => { func.instruction(&Instruction::F64Ge); }
                IrBinOp::EqF64 => { func.instruction(&Instruction::F64Eq); }
                IrBinOp::NeF64 => { func.instruction(&Instruction::F64Ne); }
                IrBinOp::And => {
                    func.instruction(&Instruction::I32And);
                }
                IrBinOp::Or => {
                    func.instruction(&Instruction::I32Or);
                }
                _ => { func.instruction(&Instruction::F64Add); } // fallback
            }
        }
        IrExpr::UnaryOp(op, operand) => {
            emit_expr(func, operand, num_imports)?;
            match op {
                IrUnaryOp::NegF64 => { func.instruction(&Instruction::F64Neg); }
                IrUnaryOp::NegI32 => {
                    func.instruction(&Instruction::I32Const(0));
                    func.instruction(&Instruction::I32Sub); // 0 - x
                }
                IrUnaryOp::Not => {
                    func.instruction(&Instruction::I32Eqz);
                }
            }
        }
        IrExpr::Call(name, args) => {
            for arg in args {
                emit_expr(func, arg, num_imports)?;
            }
            match name.as_str() {
                "math_abs" => { func.instruction(&Instruction::Call(6)); }
                "math_sqrt" => { func.instruction(&Instruction::Call(7)); }
                "math_log" => { func.instruction(&Instruction::Call(8)); }
                "math_max" => { func.instruction(&Instruction::Call(9)); }
                "math_min" => { func.instruction(&Instruction::Call(10)); }
                "set_buffer" => { func.instruction(&Instruction::Call(11)); }
                _ => {
                    // Unknown function — emit a NaN placeholder
                    func.instruction(&Instruction::F64Const(f64::NAN.into()));
                }
            }
        }
        IrExpr::F64ToI32(inner) => {
            emit_expr(func, inner, num_imports)?;
            func.instruction(&Instruction::I32TruncF64S);
        }
        IrExpr::I32ToF64(inner) => {
            emit_expr(func, inner, num_imports)?;
            func.instruction(&Instruction::F64ConvertI32S);
        }
        _ => {
            func.instruction(&Instruction::F64Const(0.0f64.into()));
        }
    }
    Ok(())
}
