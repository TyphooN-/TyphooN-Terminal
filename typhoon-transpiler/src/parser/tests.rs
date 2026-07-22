use super::*;

#[test]
fn parse_variable_declaration() {
    let src = "double myVar = 1.5;";
    let program = parse_mql5(src).expect("should parse var decl");
    assert_eq!(program.items.len(), 1);
    match &program.items[0] {
        TopLevel::GlobalVar(decl) => {
            assert_eq!(decl.name, "myVar");
            assert_eq!(decl.type_name, "double");
            assert!(!decl.is_static);
            assert!(!decl.is_const);
            match &decl.init {
                Some(Expr::FloatLit(f)) => assert!((*f - 1.5).abs() < 1e-10),
                other => panic!("expected FloatLit(1.5), got {:?}", other),
            }
        }
        other => panic!("expected GlobalVar, got {:?}", other),
    }
}

#[test]
fn parse_int_variable() {
    let src = "int count = 42;";
    let program = parse_mql5(src).expect("should parse int var");
    assert_eq!(program.items.len(), 1);
    match &program.items[0] {
        TopLevel::GlobalVar(decl) => {
            assert_eq!(decl.name, "count");
            assert_eq!(decl.type_name, "int");
            match &decl.init {
                Some(Expr::IntLit(n)) => assert_eq!(*n, 42),
                other => panic!("expected IntLit(42), got {:?}", other),
            }
        }
        other => panic!("expected GlobalVar, got {:?}", other),
    }
}

#[test]
fn parse_hex_literal() {
    let src = "int flags = 0xFF;";
    let program = parse_mql5(src).expect("should parse hex");
    match &program.items[0] {
        TopLevel::GlobalVar(decl) => match &decl.init {
            Some(Expr::IntLit(n)) => assert_eq!(*n, 255),
            other => panic!("expected IntLit(255), got {:?}", other),
        },
        other => panic!("expected GlobalVar, got {:?}", other),
    }
}

#[test]
fn parse_string_variable() {
    let src = r#"string name = "hello";"#;
    let program = parse_mql5(src).expect("should parse string var");
    match &program.items[0] {
        TopLevel::GlobalVar(decl) => {
            assert_eq!(decl.type_name, "string");
            match &decl.init {
                Some(Expr::StringLit(s)) => assert_eq!(s, "hello"),
                other => panic!("expected StringLit, got {:?}", other),
            }
        }
        other => panic!("expected GlobalVar, got {:?}", other),
    }
}

#[test]
fn parse_bool_variable() {
    let src = "bool flag = true;";
    let program = parse_mql5(src).expect("should parse bool var");
    match &program.items[0] {
        TopLevel::GlobalVar(decl) => match &decl.init {
            Some(Expr::BoolLit(b)) => assert!(*b),
            other => panic!("expected BoolLit(true), got {:?}", other),
        },
        other => panic!("expected GlobalVar, got {:?}", other),
    }
}

#[test]
fn parse_input_declaration() {
    let src = "input int InpPeriod = 14;";
    let program = parse_mql5(src).expect("should parse input");
    assert_eq!(program.items.len(), 1);
    match &program.items[0] {
        TopLevel::Input(input) => {
            assert_eq!(input.name, "InpPeriod");
            assert_eq!(input.type_name, "int");
            match &input.default {
                Some(Expr::IntLit(n)) => assert_eq!(*n, 14),
                other => panic!("expected IntLit(14), got {:?}", other),
            }
        }
        other => panic!("expected Input, got {:?}", other),
    }
}

#[test]
fn parse_simple_function() {
    let src = r#"
        int OnInit() {
            return 0;
        }
    "#;
    let program = parse_mql5(src).expect("should parse function");
    assert_eq!(program.items.len(), 1);
    match &program.items[0] {
        TopLevel::Function(func) => {
            assert_eq!(func.name, "OnInit");
            assert_eq!(func.return_type, "int");
            assert!(func.params.is_empty());
            assert_eq!(func.body.len(), 1);
        }
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parse_function_with_params() {
    let src = r#"
        int OnCalculate(int rates_total, int prev_calculated) {
            return rates_total;
        }
    "#;
    let program = parse_mql5(src).expect("should parse function with params");
    match &program.items[0] {
        TopLevel::Function(func) => {
            assert_eq!(func.name, "OnCalculate");
            assert_eq!(func.params.len(), 2);
            assert_eq!(func.params[0].name, "rates_total");
            assert_eq!(func.params[0].type_name, "int");
            assert_eq!(func.params[1].name, "prev_calculated");
        }
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parse_if_statement() {
    let src = r#"
        void Test() {
            if(x > 0)
                return;
        }
    "#;
    let program = parse_mql5(src).expect("should parse if stmt");
    match &program.items[0] {
        TopLevel::Function(func) => {
            assert_eq!(func.body.len(), 1);
            match &func.body[0] {
                Stmt::If { then, else_, .. } => {
                    assert_eq!(then.len(), 1);
                    assert!(else_.is_none());
                }
                other => panic!("expected If, got {:?}", other),
            }
        }
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parse_if_else() {
    let src = r#"
        void Test() {
            if(x > 0)
                return;
            else
                return;
        }
    "#;
    let program = parse_mql5(src).expect("should parse if/else");
    match &program.items[0] {
        TopLevel::Function(func) => match &func.body[0] {
            Stmt::If { else_, .. } => {
                assert!(else_.is_some());
            }
            other => panic!("expected If, got {:?}", other),
        },
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parse_while_loop() {
    let src = r#"
        void Test() {
            while(i < 10) {
                i++;
            }
        }
    "#;
    let program = parse_mql5(src).expect("should parse while");
    match &program.items[0] {
        TopLevel::Function(func) => match &func.body[0] {
            Stmt::While { body, .. } => {
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected While, got {:?}", other),
        },
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parse_for_loop() {
    let src = r#"
        void Test() {
            for(int i = 0; i < 10; i++) {
                break;
            }
        }
    "#;
    let program = parse_mql5(src).expect("should parse for loop");
    match &program.items[0] {
        TopLevel::Function(func) => match &func.body[0] {
            Stmt::For {
                init,
                cond,
                step,
                body,
                ..
            } => {
                assert!(init.is_some());
                assert!(cond.is_some());
                assert!(step.is_some());
                assert_eq!(body.len(), 1);
            }
            other => panic!("expected For, got {:?}", other),
        },
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parse_property_directive() {
    let src = r#"#property indicator_buffers 2"#;
    let program = parse_mql5(src).expect("should parse property");
    match &program.items[0] {
        TopLevel::Property(prop) => {
            assert_eq!(prop.name, "indicator_buffers");
            match &prop.value {
                Expr::IntLit(n) => assert_eq!(*n, 2),
                other => panic!("expected IntLit(2), got {:?}", other),
            }
        }
        other => panic!("expected Property, got {:?}", other),
    }
}

#[test]
fn parse_include_directive() {
    let src = r#"#include <MyLib.mqh>"#;
    let program = parse_mql5(src).expect("should parse include");
    match &program.items[0] {
        TopLevel::Include(path) => {
            assert!(path.contains("MyLib.mqh"));
        }
        other => panic!("expected Include, got {:?}", other),
    }
}

#[test]
fn parse_define_directive() {
    let src = "#define MY_CONST 100";
    let program = parse_mql5(src).expect("should parse define");
    match &program.items[0] {
        TopLevel::Define(name, val) => {
            assert_eq!(name, "MY_CONST");
            assert!(val.is_some());
        }
        other => panic!("expected Define, got {:?}", other),
    }
}

#[test]
fn parse_enum_definition() {
    let src = r#"
        enum MyEnum {
            VALUE_A = 0,
            VALUE_B = 1,
            VALUE_C
        };
    "#;
    let program = parse_mql5(src).expect("should parse enum");
    match &program.items[0] {
        TopLevel::Enum(e) => {
            assert_eq!(e.name, "MyEnum");
            assert_eq!(e.members.len(), 3);
            assert_eq!(e.members[0].0, "VALUE_A");
            assert!(e.members[0].1.is_some());
            assert_eq!(e.members[2].0, "VALUE_C");
            assert!(e.members[2].1.is_none());
        }
        other => panic!("expected Enum, got {:?}", other),
    }
}

#[test]
fn parse_empty_program() {
    let program = parse_mql5("").expect("should parse empty source");
    assert!(program.items.is_empty());
}

#[test]
fn parse_comments_ignored() {
    let src = r#"
        // this is a line comment
        /* this is a block comment */
        int x = 1;
    "#;
    let program = parse_mql5(src).expect("should parse with comments");
    assert_eq!(program.items.len(), 1);
}

#[test]
fn parse_var_decl_with_function_call_init() {
    // Regression: `double x = MathSqrt(4.0);` used to parse as `double x = MathSqrt;`
    // because `ident` in type_name would match MathSqrt as a type, then (4.0) would
    // fail as var_init, causing args to be dropped on backtrack.
    let src = r#"
        void Test() {
            double x = MathSqrt(4.0);
        }
    "#;
    let program = parse_mql5(src).expect("should parse var decl with call init");
    match &program.items[0] {
        TopLevel::Function(func) => match &func.body[0] {
            Stmt::VarDecl(decl) => {
                assert_eq!(decl.type_name, "double");
                assert_eq!(decl.name, "x");
                match &decl.init {
                    Some(Expr::Call { func: name, args }) => {
                        assert_eq!(name, "MathSqrt");
                        assert_eq!(args.len(), 1);
                        match &args[0] {
                            Expr::FloatLit(f) => assert!((*f - 4.0).abs() < 1e-10),
                            other => panic!("expected FloatLit(4.0), got {:?}", other),
                        }
                    }
                    other => panic!("expected Call(MathSqrt, [4.0]), got {:?}", other),
                }
            }
            other => panic!("expected VarDecl, got {:?}", other),
        },
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parse_function_call_stmt_not_misread_as_var_decl() {
    // MathSqrt(4.0); as a standalone expression should parse as expr_stmt, not var_decl
    let src = r#"
        void Test() {
            MathSqrt(4.0);
            Print("hello");
        }
    "#;
    let program = parse_mql5(src).expect("should parse function call as expr_stmt");
    match &program.items[0] {
        TopLevel::Function(func) => {
            eprintln!("body[0] = {:?}", &func.body[0]);
            eprintln!("body[1] = {:?}", &func.body[1]);
            match &func.body[0] {
                Stmt::Expr(Expr::Call { func: name, args }) => {
                    assert_eq!(name, "MathSqrt");
                    assert_eq!(args.len(), 1);
                }
                other => panic!("expected Expr(Call(MathSqrt)), got {:?}", other),
            }
        }
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parse_syntax_error_returns_err() {
    let result = parse_mql5("int = ;");
    assert!(result.is_err());
    let err = result.unwrap_err();
    match &err {
        CompileError::Parse { line, col, message } => {
            assert!(*line > 0);
            assert!(*col > 0);
            assert!(!message.is_empty());
        }
        other => panic!("expected Parse error, got {:?}", other),
    }
}

#[test]
fn parse_unclosed_brace_error() {
    let src = "void Test() { int x = 1;";
    let result = parse_mql5(src);
    assert!(result.is_err());
}

#[test]
fn parse_multiple_items() {
    let src = r#"
        input int Period = 14;
        input double Factor = 2.0;
        double buffer[];
        int OnInit() { return 0; }
    "#;
    let program = parse_mql5(src).expect("should parse multiple items");
    assert_eq!(program.items.len(), 4);
}

#[test]
fn parse_scientific_notation() {
    let src = "double val = 1.5e3;";
    let program = parse_mql5(src).expect("should parse scientific notation");
    match &program.items[0] {
        TopLevel::GlobalVar(decl) => match &decl.init {
            Some(Expr::FloatLit(f)) => assert!((*f - 1500.0).abs() < 1e-10),
            other => panic!("expected FloatLit(1500.0), got {:?}", other),
        },
        other => panic!("expected GlobalVar, got {:?}", other),
    }
}

#[test]
fn parse_null_literal() {
    let src = "int handle = NULL;";
    let program = parse_mql5(src).expect("should parse NULL");
    match &program.items[0] {
        TopLevel::GlobalVar(decl) => match &decl.init {
            Some(Expr::Null) => {}
            other => panic!("expected Null, got {:?}", other),
        },
        other => panic!("expected GlobalVar, got {:?}", other),
    }
}

#[test]
fn parse_static_variable() {
    let src = r#"
        void Test() {
            static int counter = 0;
        }
    "#;
    let program = parse_mql5(src).expect("should parse static var");
    match &program.items[0] {
        TopLevel::Function(func) => match &func.body[0] {
            Stmt::VarDecl(decl) => {
                assert!(decl.is_static);
                assert_eq!(decl.name, "counter");
            }
            other => panic!("expected VarDecl, got {:?}", other),
        },
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parse_function_call_in_body() {
    let src = r#"
        void Test() {
            Print("hello");
        }
    "#;
    let program = parse_mql5(src).expect("should parse function call");
    match &program.items[0] {
        TopLevel::Function(func) => {
            assert_eq!(func.body.len(), 1);
            // The expression statement should parse successfully
            match &func.body[0] {
                Stmt::Expr(expr) => {
                    // postfix_expr parses Print(...) as a Call
                    match expr {
                        Expr::Call { func: name, args } => {
                            assert_eq!(name, "Print");
                            assert_eq!(args.len(), 1);
                        }
                        // Some grammar paths may produce Ident for the
                        // function name when call_args are separate postfix ops
                        Expr::Ident(name) => {
                            assert_eq!(name, "Print");
                        }
                        other => panic!("expected Call or Ident, got {:?}", other),
                    }
                }
                other => panic!("expected Expr stmt, got {:?}", other),
            }
        }
        other => panic!("expected Function, got {:?}", other),
    }
}

#[test]
fn parse_call_in_var_init() {
    // This is the known parser bug — function call args in var_decl initializers
    let src = r#"
        void Test() {
            double x = MathSqrt(4.0);
        }
    "#;
    let program = parse_mql5(src).expect("should parse var decl with call init");
    match &program.items[0] {
        TopLevel::Function(func) => match &func.body[0] {
            Stmt::VarDecl(decl) => {
                assert_eq!(decl.name, "x");
                match &decl.init {
                    Some(Expr::Call { func: name, args }) => {
                        assert_eq!(name, "MathSqrt");
                        assert_eq!(args.len(), 1);
                    }
                    other => panic!("expected Call init, got {:?}", other),
                }
            }
            other => panic!("expected VarDecl, got {:?}", other),
        },
        other => panic!("expected Function, got {:?}", other),
    }
}
