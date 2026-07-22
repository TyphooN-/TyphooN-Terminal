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

fn module_with_locals(body: Vec<IrStmt>, locals: Vec<(String, IrType)>) -> IrModule {
    let mut module = module_with_body(body);
    module.on_calculate.as_mut().expect("on_calculate").locals = locals;
    module
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
    let module = module_with_locals(body, vec![("temp".into(), IrType::F64)]);
    let bytes = emit_wasm(&module).expect("should emit set_local");
    assert_eq!(&bytes[0..4], &WASM_MAGIC);
}

#[test]
fn emit_rejects_unknown_local_instead_of_using_a_scratch_slot() {
    let module = module_with_body(vec![IrStmt::SetLocal(
        "missing".into(),
        IrExpr::F64Const(1.0),
    )]);
    let err = emit_wasm(&module).expect_err("undeclared locals must not corrupt scratch slots");
    assert!(err.contains("unknown WASM local `missing`"));
}

#[test]
fn emit_assignment_expression_targets_resolved_local() {
    let body = vec![IrStmt::Expr(IrExpr::AssignLocal(
        "signal".into(),
        Box::new(IrExpr::F64Const(7.5)),
    ))];
    let module = module_with_locals(body, vec![("signal".into(), IrType::F64)]);
    let bytes = emit_wasm(&module).expect("assignment expression should emit");

    // Params occupy 0/1, scratch locals 2/3, and `signal` is local 4.
    assert!(
        bytes.windows(2).any(|window| window == [0x22, 0x04]),
        "expected local.tee 4 for the resolved assignment target"
    );
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
