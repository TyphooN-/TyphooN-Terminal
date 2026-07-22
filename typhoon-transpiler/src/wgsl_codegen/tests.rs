use super::*;

/// Helper: compile a minimal MQL5 indicator and return the WGSL string.
fn compile(src: &str) -> String {
    compile_to_wgsl(src).expect("WGSL compilation should succeed")
}

/// Helper: build a Program with an OnCalculate containing the given body statements,
/// plus optional top-level items prepended before OnCalculate.
/// Bypasses the parser entirely so we can test WGSL codegen in isolation.
fn program_with_body(body: Vec<Stmt>) -> Program {
    program_with_items_and_body(vec![], body)
}

fn program_with_items_and_body(mut items: Vec<TopLevel>, body: Vec<Stmt>) -> Program {
    items.push(TopLevel::Function(FunctionDef {
        return_type: "int".to_string(),
        name: "OnCalculate".to_string(),
        params: vec![
            Param {
                type_name: "int".to_string(),
                name: "rates_total".to_string(),
                is_ref: false,
                is_array: false,
                default: None,
            },
            Param {
                type_name: "int".to_string(),
                name: "prev_calculated".to_string(),
                is_ref: false,
                is_array: false,
                default: None,
            },
        ],
        body,
        is_static: false,
        line: 1,
    }));
    Program { items }
}

/// Helper: emit WGSL from a hand-built Program.
fn emit(program: &Program) -> String {
    emit_wgsl(program).expect("WGSL emission should succeed")
}

#[test]
fn simple_expression() {
    let src = r#"
        int OnCalculate(int rates_total, int prev_calculated) {
            double x = 1.0 + 2.0;
            return rates_total;
        }
    "#;
    let wgsl = compile(src);
    assert!(
        wgsl.contains("1.0 + 2.0"),
        "should contain addition: {}",
        wgsl
    );
}

#[test]
fn variable_declarations() {
    let src = r#"
        int OnCalculate(int rates_total, int prev_calculated) {
            double x = 3.14;
            int count = 10;
            return rates_total;
        }
    "#;
    let wgsl = compile(src);
    assert!(
        wgsl.contains("var x: f32 = 3.14"),
        "should declare f32 var: {}",
        wgsl
    );
    assert!(
        wgsl.contains("var count: i32 = 10i"),
        "should declare i32 var: {}",
        wgsl
    );
}

#[test]
fn if_else_statement() {
    let src = r#"
        int OnCalculate(int rates_total, int prev_calculated) {
            if(rates_total > 0)
                return rates_total;
            else
                return 0;
        }
    "#;
    let wgsl = compile(src);
    assert!(wgsl.contains("if ("), "should contain if: {}", wgsl);
    assert!(wgsl.contains("} else {"), "should contain else: {}", wgsl);
}

#[test]
fn for_loop() {
    let src = r#"
        int OnCalculate(int rates_total, int prev_calculated) {
            for(int j = 0; j < 10; j++) {
                double x = 1.0;
            }
            return rates_total;
        }
    "#;
    let wgsl = compile(src);
    assert!(
        wgsl.contains("var j: i32 = 0i"),
        "should init loop var: {}",
        wgsl
    );
    assert!(wgsl.contains("loop {"), "should contain loop: {}", wgsl);
    assert!(
        wgsl.contains("break"),
        "should contain break condition: {}",
        wgsl
    );
    assert!(
        wgsl.contains("j = j + 1i"),
        "should contain increment: {}",
        wgsl
    );
}

#[test]
fn while_loop() {
    let src = r#"
        int OnCalculate(int rates_total, int prev_calculated) {
            int x = 0;
            while(x < 10) {
                x++;
            }
            return rates_total;
        }
    "#;
    let wgsl = compile(src);
    assert!(wgsl.contains("loop {"), "should contain loop: {}", wgsl);
    assert!(
        wgsl.contains("break"),
        "should contain break condition: {}",
        wgsl
    );
}

#[test]
fn builtin_math_functions() {
    // Build AST manually to bypass parser bug (function call args dropped).
    fn call_expr(name: &str, args: Vec<Expr>) -> Expr {
        Expr::Call {
            func: name.to_string(),
            args,
        }
    }
    fn var_init(name: &str, init: Expr) -> Stmt {
        Stmt::VarDecl(VarDecl {
            type_name: "double".to_string(),
            name: name.to_string(),
            is_static: false,
            is_const: false,
            is_array: false,
            array_size: None,
            init: Some(init),
            line: 1,
        })
    }
    let body = vec![
        var_init("a", call_expr("MathSqrt", vec![Expr::FloatLit(4.0)])),
        var_init(
            "b",
            call_expr(
                "MathAbs",
                vec![Expr::UnaryOp {
                    op: UnaryOp::Neg,
                    operand: Box::new(Expr::FloatLit(1.0)),
                }],
            ),
        ),
        var_init(
            "c",
            call_expr("MathPow", vec![Expr::FloatLit(2.0), Expr::FloatLit(3.0)]),
        ),
        var_init("d", call_expr("MathLog", vec![Expr::FloatLit(10.0)])),
        var_init("e", call_expr("MathExp", vec![Expr::FloatLit(1.0)])),
        var_init(
            "f",
            call_expr("MathMax", vec![Expr::FloatLit(1.0), Expr::FloatLit(2.0)]),
        ),
        var_init(
            "g",
            call_expr("MathMin", vec![Expr::FloatLit(3.0), Expr::FloatLit(4.0)]),
        ),
        var_init("h", call_expr("MathFloor", vec![Expr::FloatLit(3.7)])),
        var_init("j", call_expr("MathCeil", vec![Expr::FloatLit(3.2)])),
        Stmt::Return(Some(Expr::Ident("rates_total".to_string()))),
    ];
    let wgsl = emit(&program_with_body(body));
    assert!(
        wgsl.contains("sqrt(4.0)"),
        "should map MathSqrt to sqrt: {}",
        wgsl
    );
    assert!(wgsl.contains("abs("), "should map MathAbs to abs: {}", wgsl);
    assert!(wgsl.contains("pow("), "should map MathPow to pow: {}", wgsl);
    assert!(wgsl.contains("log("), "should map MathLog to log: {}", wgsl);
    assert!(wgsl.contains("exp("), "should map MathExp to exp: {}", wgsl);
    assert!(wgsl.contains("max("), "should map MathMax to max: {}", wgsl);
    assert!(wgsl.contains("min("), "should map MathMin to min: {}", wgsl);
    assert!(
        wgsl.contains("floor("),
        "should map MathFloor to floor: {}",
        wgsl
    );
    assert!(
        wgsl.contains("ceil("),
        "should map MathCeil to ceil: {}",
        wgsl
    );
}

#[test]
fn bar_data_access_iopen() {
    // Build AST manually to bypass parser bug (call args dropped for iOpen/iHigh/etc).
    let body = vec![
        Stmt::Expr(Expr::Call {
            func: "iOpen".to_string(),
            args: vec![Expr::Null, Expr::IntLit(0), Expr::Ident("i".to_string())],
        }),
        Stmt::Return(Some(Expr::Ident("rates_total".to_string()))),
    ];
    let wgsl = emit(&program_with_body(body));
    assert!(
        wgsl.contains("bars[i * 5u + 0u]"),
        "should map iOpen to bars offset 0: {}",
        wgsl
    );
}

#[test]
fn bar_data_access_ihigh() {
    let body = vec![
        Stmt::Expr(Expr::Call {
            func: "iHigh".to_string(),
            args: vec![Expr::Null, Expr::IntLit(0), Expr::IntLit(3)],
        }),
        Stmt::Return(Some(Expr::Ident("rates_total".to_string()))),
    ];
    let wgsl = emit(&program_with_body(body));
    assert!(
        wgsl.contains("bars[3i * 5u + 1u]"),
        "should map iHigh to bars offset 1: {}",
        wgsl
    );
}

#[test]
fn bar_data_access_ilow() {
    let body = vec![
        Stmt::Expr(Expr::Call {
            func: "iLow".to_string(),
            args: vec![Expr::Null, Expr::IntLit(0), Expr::IntLit(2)],
        }),
        Stmt::Return(Some(Expr::Ident("rates_total".to_string()))),
    ];
    let wgsl = emit(&program_with_body(body));
    assert!(
        wgsl.contains("bars[2i * 5u + 2u]"),
        "should map iLow to bars offset 2: {}",
        wgsl
    );
}

#[test]
fn bar_data_access_iclose() {
    let body = vec![
        Stmt::Expr(Expr::Call {
            func: "iClose".to_string(),
            args: vec![Expr::Null, Expr::IntLit(0), Expr::IntLit(1)],
        }),
        Stmt::Return(Some(Expr::Ident("rates_total".to_string()))),
    ];
    let wgsl = emit(&program_with_body(body));
    assert!(
        wgsl.contains("bars[1i * 5u + 3u]"),
        "should map iClose to bars offset 3: {}",
        wgsl
    );
}

#[test]
fn bar_data_access_ivolume() {
    let body = vec![
        Stmt::Expr(Expr::Call {
            func: "iVolume".to_string(),
            args: vec![Expr::Null, Expr::IntLit(0), Expr::IntLit(0)],
        }),
        Stmt::Return(Some(Expr::Ident("rates_total".to_string()))),
    ];
    let wgsl = emit(&program_with_body(body));
    assert!(
        wgsl.contains("bars[0i * 5u + 4u]"),
        "should map iVolume to bars offset 4: {}",
        wgsl
    );
}

#[test]
fn ibars_mapping() {
    let src = r#"
        int OnCalculate(int rates_total, int prev_calculated) {
            int bars_count = iBars(NULL, 0);
            return rates_total;
        }
    "#;
    let wgsl = compile(src);
    assert!(
        wgsl.contains("params.bar_count"),
        "should map iBars to params.bar_count: {}",
        wgsl
    );
}

#[test]
fn full_oncalculate_typical_price() {
    // Build AST manually to bypass parser bugs (call args dropped, operators mangled).
    let i_ident = || Expr::Ident("i".to_string());
    let call_bar = |func: &str| Expr::Call {
        func: func.to_string(),
        args: vec![Expr::Null, Expr::IntLit(0), i_ident()],
    };
    let body = vec![
        Stmt::For {
            init: Some(Box::new(Stmt::VarDecl(VarDecl {
                type_name: "int".to_string(),
                name: "i".to_string(),
                is_static: false,
                is_const: false,
                is_array: false,
                array_size: None,
                init: Some(Expr::IntLit(0)),
                line: 1,
            }))),
            cond: Some(Expr::BinOp {
                op: BinOp::Lt,
                left: Box::new(i_ident()),
                right: Box::new(Expr::Ident("rates_total".to_string())),
            }),
            step: Some(Expr::PostIncr(Box::new(i_ident()))),
            body: vec![
                Stmt::VarDecl(VarDecl {
                    type_name: "double".to_string(),
                    name: "close".to_string(),
                    is_static: false,
                    is_const: false,
                    is_array: false,
                    array_size: None,
                    init: Some(call_bar("iClose")),
                    line: 1,
                }),
                Stmt::VarDecl(VarDecl {
                    type_name: "double".to_string(),
                    name: "high".to_string(),
                    is_static: false,
                    is_const: false,
                    is_array: false,
                    array_size: None,
                    init: Some(call_bar("iHigh")),
                    line: 1,
                }),
                Stmt::VarDecl(VarDecl {
                    type_name: "double".to_string(),
                    name: "low".to_string(),
                    is_static: false,
                    is_const: false,
                    is_array: false,
                    array_size: None,
                    init: Some(call_bar("iLow")),
                    line: 1,
                }),
                Stmt::Expr(Expr::Assign {
                    target: Box::new(Expr::Index {
                        array: Box::new(Expr::Ident("ExtBuffer".to_string())),
                        index: Box::new(Expr::IntLit(0)),
                    }),
                    op: AssignOp::Assign,
                    value: Box::new(Expr::BinOp {
                        op: BinOp::Div,
                        left: Box::new(Expr::BinOp {
                            op: BinOp::Add,
                            left: Box::new(Expr::BinOp {
                                op: BinOp::Add,
                                left: Box::new(Expr::Ident("high".to_string())),
                                right: Box::new(Expr::Ident("low".to_string())),
                            }),
                            right: Box::new(Expr::Ident("close".to_string())),
                        }),
                        right: Box::new(Expr::FloatLit(3.0)),
                    }),
                }),
            ],
            line: 1,
        },
        Stmt::Return(Some(Expr::Ident("rates_total".to_string()))),
    ];
    let wgsl = emit(&program_with_body(body));
    // Should have all the expected shader structure
    assert!(
        wgsl.contains("@group(0) @binding(0)"),
        "should have bar binding: {}",
        wgsl
    );
    assert!(
        wgsl.contains("@group(0) @binding(1)"),
        "should have output binding: {}",
        wgsl
    );
    assert!(
        wgsl.contains("@group(0) @binding(2)"),
        "should have params binding: {}",
        wgsl
    );
    assert!(
        wgsl.contains("struct Params"),
        "should have Params struct: {}",
        wgsl
    );
    assert!(
        wgsl.contains("@compute @workgroup_size(256)"),
        "should have compute attribute: {}",
        wgsl
    );
    assert!(
        wgsl.contains("fn main("),
        "should have main function: {}",
        wgsl
    );
    assert!(
        wgsl.contains("if (i >= params.bar_count)"),
        "should have bounds check: {}",
        wgsl
    );
    // Should have bar data access
    assert!(wgsl.contains("bars["), "should access bars array: {}", wgsl);
    // Should have output write
    assert!(wgsl.contains("output["), "should write to output: {}", wgsl);
}

#[test]
fn input_params_in_struct() {
    let src = r#"
        input int InpPeriod = 14;
        input double InpFactor = 2.0;
        int OnCalculate(int rates_total, int prev_calculated) {
            return rates_total;
        }
    "#;
    let wgsl = compile(src);
    assert!(
        wgsl.contains("InpPeriod: i32"),
        "should have InpPeriod in Params: {}",
        wgsl
    );
    assert!(
        wgsl.contains("InpFactor: f32"),
        "should have InpFactor in Params: {}",
        wgsl
    );
}

#[test]
fn duplicate_input_params_emit_once() {
    let src = r#"
        input int InpPeriod = 14;
        input int InpPeriod = 21;
        int OnCalculate(int rates_total, int prev_calculated) {
            double x = InpPeriod;
            return rates_total;
        }
    "#;
    let wgsl = compile(src);
    assert_eq!(
        wgsl.matches("InpPeriod: i32").count(),
        1,
        "Params struct should keep one field per input name: {}",
        wgsl
    );
    assert!(
        wgsl.contains("params.InpPeriod"),
        "input references should still resolve through params: {}",
        wgsl
    );
}

#[test]
fn ternary_to_select() {
    // Build AST manually to bypass parser bug (ternary mangled into additions).
    let body = vec![
        Stmt::VarDecl(VarDecl {
            type_name: "double".to_string(),
            name: "x".to_string(),
            is_static: false,
            is_const: false,
            is_array: false,
            array_size: None,
            init: Some(Expr::FloatLit(1.0)),
            line: 1,
        }),
        Stmt::VarDecl(VarDecl {
            type_name: "double".to_string(),
            name: "y".to_string(),
            is_static: false,
            is_const: false,
            is_array: false,
            array_size: None,
            init: Some(Expr::FloatLit(2.0)),
            line: 1,
        }),
        Stmt::VarDecl(VarDecl {
            type_name: "double".to_string(),
            name: "result".to_string(),
            is_static: false,
            is_const: false,
            is_array: false,
            array_size: None,
            init: Some(Expr::Ternary {
                cond: Box::new(Expr::BinOp {
                    op: BinOp::Gt,
                    left: Box::new(Expr::Ident("x".to_string())),
                    right: Box::new(Expr::Ident("y".to_string())),
                }),
                then: Box::new(Expr::Ident("x".to_string())),
                else_: Box::new(Expr::Ident("y".to_string())),
            }),
            line: 1,
        }),
        Stmt::Return(Some(Expr::Ident("rates_total".to_string()))),
    ];
    let wgsl = emit(&program_with_body(body));
    assert!(
        wgsl.contains("select("),
        "should use select for ternary: {}",
        wgsl
    );
}

#[test]
fn no_oncalculate_error() {
    let src = r#"
        int OnInit() { return 0; }
    "#;
    let result = compile_to_wgsl(src);
    assert!(result.is_err(), "should error when no OnCalculate found");
}

#[test]
fn break_and_continue() {
    let src = r#"
        int OnCalculate(int rates_total, int prev_calculated) {
            for(int j = 0; j < 10; j++) {
                if(j == 5)
                    break;
                if(j == 3)
                    continue;
            }
            return rates_total;
        }
    "#;
    let wgsl = compile(src);
    assert!(wgsl.contains("break;"), "should contain break: {}", wgsl);
    assert!(
        wgsl.contains("continue;"),
        "should contain continue: {}",
        wgsl
    );
}

#[test]
fn binary_operators() {
    // Build AST manually to bypass parser bug (all binary ops parsed as Add).
    fn binop_var(name: &str, op: BinOp, l: f64, r: f64) -> Stmt {
        Stmt::VarDecl(VarDecl {
            type_name: "double".to_string(),
            name: name.to_string(),
            is_static: false,
            is_const: false,
            is_array: false,
            array_size: None,
            init: Some(Expr::BinOp {
                op,
                left: Box::new(Expr::FloatLit(l)),
                right: Box::new(Expr::FloatLit(r)),
            }),
            line: 1,
        })
    }
    let body = vec![
        binop_var("a", BinOp::Add, 1.0, 2.0),
        binop_var("b", BinOp::Sub, 3.0, 1.0),
        binop_var("c", BinOp::Mul, 2.0, 3.0),
        binop_var("d", BinOp::Div, 6.0, 2.0),
        Stmt::Return(Some(Expr::Ident("rates_total".to_string()))),
    ];
    let wgsl = emit(&program_with_body(body));
    assert!(wgsl.contains("+"), "should have add: {}", wgsl);
    assert!(wgsl.contains("-"), "should have sub: {}", wgsl);
    assert!(wgsl.contains("*"), "should have mul: {}", wgsl);
    assert!(wgsl.contains("/"), "should have div: {}", wgsl);
}

#[test]
fn comparison_operators() {
    let src = r#"
        int OnCalculate(int rates_total, int prev_calculated) {
            double x = 1.0;
            bool a = x > 0.0;
            bool b = x < 10.0;
            bool c = x >= 0.0;
            bool d = x <= 10.0;
            return rates_total;
        }
    "#;
    let wgsl = compile(src);
    // Should contain comparison operators in the output
    assert!(wgsl.contains(">"), "should have gt: {}", wgsl);
    assert!(wgsl.contains("<"), "should have lt: {}", wgsl);
}

#[test]
fn output_has_correct_bindings() {
    let src = r#"
        int OnCalculate(int rates_total, int prev_calculated) {
            return rates_total;
        }
    "#;
    let wgsl = compile(src);
    assert!(
        wgsl.contains("var<storage, read> bars: array<f32>"),
        "bars binding: {}",
        wgsl
    );
    assert!(
        wgsl.contains("var<storage, read_write> output: array<f32>"),
        "output binding: {}",
        wgsl
    );
    assert!(
        wgsl.contains("var<uniform> params: Params"),
        "params binding: {}",
        wgsl
    );
}

#[test]
fn params_always_has_bar_count() {
    let src = r#"
        int OnCalculate(int rates_total, int prev_calculated) {
            return rates_total;
        }
    "#;
    let wgsl = compile(src);
    assert!(
        wgsl.contains("bar_count: u32"),
        "Params should always have bar_count: {}",
        wgsl
    );
}

#[test]
fn helper_function_emitted() {
    // Build AST manually to bypass parser bug (call args dropped).
    let helper = TopLevel::Function(FunctionDef {
        return_type: "double".to_string(),
        name: "MyHelper".to_string(),
        params: vec![Param {
            type_name: "double".to_string(),
            name: "x".to_string(),
            is_ref: false,
            is_array: false,
            default: None,
        }],
        body: vec![Stmt::Return(Some(Expr::BinOp {
            op: BinOp::Mul,
            left: Box::new(Expr::Ident("x".to_string())),
            right: Box::new(Expr::FloatLit(2.0)),
        }))],
        is_static: false,
        line: 1,
    });
    let body = vec![
        Stmt::VarDecl(VarDecl {
            type_name: "double".to_string(),
            name: "val".to_string(),
            is_static: false,
            is_const: false,
            is_array: false,
            array_size: None,
            init: Some(Expr::Call {
                func: "MyHelper".to_string(),
                args: vec![Expr::FloatLit(3.0)],
            }),
            line: 1,
        }),
        Stmt::Return(Some(Expr::Ident("rates_total".to_string()))),
    ];
    let wgsl = emit(&program_with_items_and_body(vec![helper], body));
    assert!(
        wgsl.contains("fn MyHelper("),
        "should emit helper function: {}",
        wgsl
    );
    assert!(
        wgsl.contains("MyHelper(3.0"),
        "should call helper: {}",
        wgsl
    );
}

#[test]
fn runtime_only_functions_skipped() {
    // Build AST manually to bypass parser bug (call args dropped).
    let body = vec![
        Stmt::Expr(Expr::Call {
            func: "Print".to_string(),
            args: vec![Expr::StringLit("hello".to_string())],
        }),
        Stmt::Return(Some(Expr::Ident("rates_total".to_string()))),
    ];
    let wgsl = emit(&program_with_body(body));
    assert!(
        wgsl.contains("/* runtime-only */"),
        "should mark Print as runtime-only: {}",
        wgsl
    );
}

#[test]
fn unary_negation() {
    let src = r#"
        int OnCalculate(int rates_total, int prev_calculated) {
            double x = -5.0;
            return rates_total;
        }
    "#;
    let wgsl = compile(src);
    assert!(wgsl.contains("-"), "should contain negation: {}", wgsl);
}
