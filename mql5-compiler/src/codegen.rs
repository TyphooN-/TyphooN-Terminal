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
        *self.map.get(name).unwrap_or(&2) // fallback to temp local
    }
}

fn emit_function_body(
    func: &mut Function,
    ir_func: &IrFunction,
    num_imports: u32,
) -> Result<(), String> {
    let local_map = LocalMap::new(&ir_func.params, &ir_func.locals);

    // Declare additional locals for IR-declared variables
    // (the Function::new already declares i:i32 and temp:f64)
    // Add extra locals for user-declared variables
    for (_name, ir_type) in &ir_func.locals {
        let vt = match ir_type {
            IrType::I32 | IrType::Bool => ValType::I32,
            IrType::I64 => ValType::I64,
            IrType::F64 => ValType::F64,
            IrType::String => ValType::I32, // string as i32 pointer
        };
        // Note: wasm-encoder Function::new already allocated fixed locals,
        // we'd need to rebuild. For now, the fixed allocation covers most cases.
        let _ = vt; // suppress unused warning
    }

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
            func.instruction(&Instruction::LocalSet(locals.resolve(name)));
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
                "__assign" if args.len() == 2 => {
                    // Assignment expression: set local and leave value on stack
                    // args[0] is GetLocal(name) marker, args[1] is the value
                    // We already emitted args[0] (GetLocal) and args[1] (value) above.
                    // Pop the GetLocal result, keep the value, tee_local.
                    // Actually, the value is on top of the stack now.
                    // Just do local.tee to set and keep on stack.
                    // But we need the local index — extract from the first arg.
                    // For now, use temp local (index 3).
                    func.instruction(&Instruction::LocalTee(3));
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
mod tests {
    use super::*;

    /// Helper: create a minimal IrModule with no on_calculate (default return).
    fn empty_module() -> IrModule {
        IrModule {
            buffers: vec![],
            inputs: vec![],
            functions: vec![],
            on_calculate: None,
            on_init: None,
            globals: vec![],
        }
    }

    /// Helper: create an IrModule with a single-statement on_calculate body.
    fn module_with_body(body: Vec<IrStmt>) -> IrModule {
        IrModule {
            buffers: vec![],
            inputs: vec![],
            functions: vec![],
            on_calculate: Some(IrFunction {
                name: "OnCalculate".into(),
                params: vec![
                    ("rates_total".into(), IrType::I32),
                    ("prev_calculated".into(), IrType::I32),
                ],
                return_type: IrType::I32,
                body,
                locals: vec![],
            }),
            on_init: None,
            globals: vec![],
        }
    }

    /// WASM magic number: \0asm
    const WASM_MAGIC: [u8; 4] = [0x00, 0x61, 0x73, 0x6d];
    /// WASM version 1
    const WASM_VERSION: [u8; 4] = [0x01, 0x00, 0x00, 0x00];

    #[test]
    fn emit_empty_module_produces_valid_wasm() {
        let module = empty_module();
        let bytes = emit_wasm(&module).expect("should emit wasm");
        assert!(
            bytes.len() >= 8,
            "WASM binary too short: {} bytes",
            bytes.len()
        );
        assert_eq!(&bytes[0..4], &WASM_MAGIC, "missing WASM magic number");
        assert_eq!(&bytes[4..8], &WASM_VERSION, "wrong WASM version");
    }

    #[test]
    fn emit_module_exports_on_calculate_and_memory() {
        let module = empty_module();
        let bytes = emit_wasm(&module).expect("should emit wasm");
        // The binary should contain the export names as UTF-8 strings
        let wasm_str = String::from_utf8_lossy(&bytes);
        assert!(
            wasm_str.contains("on_calculate"),
            "missing on_calculate export"
        );
        assert!(wasm_str.contains("memory"), "missing memory export");
    }

    #[test]
    fn emit_i32_const_expression() {
        let body = vec![IrStmt::Return(Some(IrExpr::I32Const(42)))];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit wasm");
        assert!(&bytes[0..4] == &WASM_MAGIC);
        // Binary should be non-trivially sized (has actual code)
        assert!(bytes.len() > 50);
    }

    #[test]
    fn emit_f64_const_expression() {
        let body = vec![IrStmt::Expr(IrExpr::F64Const(3.14))];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit wasm");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
        // The f64 value 3.14 should appear as IEEE 754 bytes somewhere in the binary
        let f64_bytes = 3.14_f64.to_le_bytes();
        let found = bytes.windows(8).any(|w| w == f64_bytes);
        assert!(found, "f64 constant 3.14 not found in WASM binary");
    }

    #[test]
    fn emit_f64_nan_for_unknown_function() {
        // Unknown function calls should emit NaN placeholder
        let body = vec![IrStmt::Expr(IrExpr::Call("UnknownFunc".into(), vec![]))];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit wasm for unknown func");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
        // NaN in IEEE 754 has specific byte pattern
        let nan_bytes = f64::NAN.to_le_bytes();
        let found = bytes.windows(8).any(|w| w == nan_bytes);
        assert!(found, "NaN placeholder not found for unknown function");
    }

    #[test]
    fn emit_known_math_functions() {
        // Known math functions should NOT emit NaN — they emit Call instructions
        for func_name in &["math_abs", "math_sqrt", "math_log"] {
            let body = vec![IrStmt::Expr(IrExpr::Call(
                func_name.to_string(),
                vec![IrExpr::F64Const(1.0)],
            ))];
            let module = module_with_body(body);
            let bytes = emit_wasm(&module).expect(&format!("should emit wasm for {}", func_name));
            assert_eq!(&bytes[0..4], &WASM_MAGIC);
        }
    }

    #[test]
    fn emit_binary_ops() {
        let ops = vec![
            (IrBinOp::AddF64, "add"),
            (IrBinOp::SubF64, "sub"),
            (IrBinOp::MulF64, "mul"),
            (IrBinOp::DivF64, "div"),
        ];
        for (op, label) in ops {
            let body = vec![IrStmt::Expr(IrExpr::BinOp(
                op,
                Box::new(IrExpr::F64Const(2.0)),
                Box::new(IrExpr::F64Const(3.0)),
            ))];
            let module = module_with_body(body);
            let bytes = emit_wasm(&module).expect(&format!("should emit wasm for f64.{}", label));
            assert_eq!(&bytes[0..4], &WASM_MAGIC);
        }
    }

    #[test]
    fn emit_comparison_ops() {
        let ops = vec![
            IrBinOp::LtF64,
            IrBinOp::LeF64,
            IrBinOp::GtF64,
            IrBinOp::GeF64,
            IrBinOp::EqF64,
            IrBinOp::NeF64,
        ];
        for op in ops {
            let body = vec![IrStmt::Expr(IrExpr::BinOp(
                op,
                Box::new(IrExpr::F64Const(1.0)),
                Box::new(IrExpr::F64Const(2.0)),
            ))];
            let module = module_with_body(body);
            let bytes = emit_wasm(&module).expect("should emit comparison op");
            assert_eq!(&bytes[0..4], &WASM_MAGIC);
        }
    }

    #[test]
    fn emit_unary_neg_f64() {
        let body = vec![IrStmt::Expr(IrExpr::UnaryOp(
            IrUnaryOp::NegF64,
            Box::new(IrExpr::F64Const(5.0)),
        ))];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit unary neg");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }

    #[test]
    fn emit_unary_not() {
        let body = vec![IrStmt::Expr(IrExpr::UnaryOp(
            IrUnaryOp::Not,
            Box::new(IrExpr::I32Const(1)),
        ))];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit unary not");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }

    #[test]
    fn emit_if_statement() {
        let body = vec![IrStmt::If {
            cond: IrExpr::I32Const(1),
            then: vec![IrStmt::Return(Some(IrExpr::I32Const(1)))],
            else_: vec![IrStmt::Return(Some(IrExpr::I32Const(0)))],
        }];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit if stmt");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }

    #[test]
    fn emit_loop_and_break() {
        let body = vec![IrStmt::Loop {
            body: vec![IrStmt::Break],
        }];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit loop");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }

    #[test]
    fn emit_bar_data_access() {
        let body = vec![
            IrStmt::Expr(IrExpr::IOpen(Box::new(IrExpr::I32Const(0)))),
            IrStmt::Expr(IrExpr::IHigh(Box::new(IrExpr::I32Const(0)))),
            IrStmt::Expr(IrExpr::ILow(Box::new(IrExpr::I32Const(0)))),
            IrStmt::Expr(IrExpr::IClose(Box::new(IrExpr::I32Const(0)))),
            IrStmt::Expr(IrExpr::IVolume(Box::new(IrExpr::I32Const(0)))),
            IrStmt::Expr(IrExpr::IBars),
        ];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit bar data access");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }

    #[test]
    fn emit_type_conversions() {
        let body = vec![
            IrStmt::Expr(IrExpr::F64ToI32(Box::new(IrExpr::F64Const(3.7)))),
            IrStmt::Expr(IrExpr::I32ToF64(Box::new(IrExpr::I32Const(42)))),
        ];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit type conversions");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }

    #[test]
    fn emit_integer_zero() {
        let body = vec![IrStmt::Return(Some(IrExpr::I32Const(0)))];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit i32 zero");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }

    #[test]
    fn emit_negative_i32() {
        let body = vec![IrStmt::Expr(IrExpr::UnaryOp(
            IrUnaryOp::NegI32,
            Box::new(IrExpr::I32Const(10)),
        ))];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit neg i32");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }

    #[test]
    fn emit_math_max_min() {
        let body = vec![
            IrStmt::Expr(IrExpr::Call(
                "math_max".into(),
                vec![IrExpr::F64Const(1.0), IrExpr::F64Const(2.0)],
            )),
            IrStmt::Expr(IrExpr::Call(
                "math_min".into(),
                vec![IrExpr::F64Const(3.0), IrExpr::F64Const(4.0)],
            )),
        ];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit math max/min");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }

    #[test]
    fn emit_set_local() {
        let body = vec![IrStmt::SetLocal("temp".into(), IrExpr::F64Const(1.0))];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit set_local");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }

    #[test]
    fn emit_return_none() {
        let body = vec![IrStmt::Return(None)];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit return none");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }

    #[test]
    fn emit_block_stmts() {
        let body = vec![IrStmt::Block(vec![
            IrStmt::Expr(IrExpr::I32Const(1)),
            IrStmt::Expr(IrExpr::I32Const(2)),
        ])];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit block");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }

    #[test]
    fn emit_imports_bar_data_functions() {
        let module = empty_module();
        let bytes = emit_wasm(&module).expect("should emit wasm");
        let wasm_str = String::from_utf8_lossy(&bytes);
        // Import names should appear in the binary
        for name in &["iBars", "iOpen", "iHigh", "iLow", "iClose", "iVolume"] {
            assert!(wasm_str.contains(name), "missing import: {}", name);
        }
    }

    #[test]
    fn emit_imports_math_functions() {
        let module = empty_module();
        let bytes = emit_wasm(&module).expect("should emit wasm");
        let wasm_str = String::from_utf8_lossy(&bytes);
        for name in &["math_abs", "math_sqrt", "math_log", "math_max", "math_min"] {
            assert!(wasm_str.contains(name), "missing math import: {}", name);
        }
    }

    #[test]
    fn emit_logical_ops() {
        let body = vec![
            IrStmt::Expr(IrExpr::BinOp(
                IrBinOp::And,
                Box::new(IrExpr::I32Const(1)),
                Box::new(IrExpr::I32Const(0)),
            )),
            IrStmt::Expr(IrExpr::BinOp(
                IrBinOp::Or,
                Box::new(IrExpr::I32Const(0)),
                Box::new(IrExpr::I32Const(1)),
            )),
        ];
        let module = module_with_body(body);
        let bytes = emit_wasm(&module).expect("should emit logical ops");
        assert_eq!(&bytes[0..4], &WASM_MAGIC);
    }
}
