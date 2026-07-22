//! MQL5 Parser — pest grammar → AST.
//!
//! Zero `.unwrap()` in production code — all iterator advances use `next_or_err()`
//! which returns `CompileError::Internal` if a grammar-guaranteed child is missing.
//! This can only happen if the pest grammar and parser get out of sync, in which
//! case a clear error message is far better than a panic.

use pest::Parser;
use pest_derive::Parser;

use crate::ast::*;
use crate::error::CompileError;

#[derive(Parser)]
#[grammar = "mql5.pest"]
struct Mql5Parser;

/// Advance a pest `Pairs` iterator, returning `CompileError::Internal` if empty.
/// Replaces all `.next().unwrap()` calls with proper error propagation.
fn next_or_err<'a>(
    iter: &mut pest::iterators::Pairs<'a, Rule>,
    context: &str,
) -> Result<pest::iterators::Pair<'a, Rule>, CompileError> {
    iter.next().ok_or_else(|| {
        CompileError::Internal(format!(
            "parser: expected child in {context}, got none — grammar/parser mismatch"
        ))
    })
}

/// Parse MQL5 source into an AST.
pub fn parse_mql5(source: &str) -> Result<Program, CompileError> {
    let pairs = Mql5Parser::parse(Rule::program, source).map_err(|e| {
        let (line, col) = match e.line_col {
            pest::error::LineColLocation::Pos((l, c)) => (l, c),
            pest::error::LineColLocation::Span((l, c), _) => (l, c),
        };
        CompileError::Parse {
            message: format!("{e}"),
            line,
            col,
        }
    })?;

    let mut items = Vec::new();
    for pair in pairs {
        match pair.as_rule() {
            Rule::program => {
                for inner in pair.into_inner() {
                    if let Some(item) = parse_top_level(inner)? {
                        items.push(item);
                    }
                }
            }
            _ => {}
        }
    }

    Ok(Program { items })
}

fn parse_top_level(
    pair: pest::iterators::Pair<'_, Rule>,
) -> Result<Option<TopLevel>, CompileError> {
    let line = pair.line_col().0;
    match pair.as_rule() {
        Rule::property_directive => {
            let mut inner = pair.into_inner();
            let name = next_or_err(&mut inner, "property_directive name")?
                .as_str()
                .to_string();
            let value = if let Some(expr_pair) = inner.next() {
                parse_expr(expr_pair)?
            } else {
                Expr::BoolLit(true)
            };
            Ok(Some(TopLevel::Property(Property { name, value, line })))
        }
        Rule::preprocessor => {
            let inner = next_or_err(&mut pair.into_inner(), "preprocessor")?;
            match inner.as_rule() {
                Rule::include_dir => {
                    let path = next_or_err(&mut inner.into_inner(), "include_dir path")?
                        .as_str()
                        .to_string();
                    Ok(Some(TopLevel::Include(path)))
                }
                Rule::define_dir => {
                    let mut parts = inner.into_inner();
                    let name = next_or_err(&mut parts, "define_dir name")?
                        .as_str()
                        .to_string();
                    let value = parts.next().map(|p| parse_expr(p)).transpose()?;
                    Ok(Some(TopLevel::Define(name, value)))
                }
                _ => Ok(None), // ifdef/endif/else — preprocessor flow control (skip for now)
            }
        }
        Rule::global_decl => {
            let inner = next_or_err(&mut pair.into_inner(), "global_decl")?;
            match inner.as_rule() {
                Rule::input_decl => {
                    let mut parts = inner.into_inner();
                    let type_name = parse_type_spec(next_or_err(&mut parts, "input_decl type")?);
                    let name = next_or_err(&mut parts, "input_decl name")?
                        .as_str()
                        .to_string();
                    let default = parts.next().map(|p| parse_expr(p)).transpose()?;
                    Ok(Some(TopLevel::Input(InputDecl {
                        type_name,
                        name,
                        default,
                        line,
                    })))
                }
                Rule::sinput_decl => {
                    let mut parts = inner.into_inner();
                    let type_name = parse_type_spec(next_or_err(&mut parts, "sinput_decl type")?);
                    let name = next_or_err(&mut parts, "sinput_decl name")?
                        .as_str()
                        .to_string();
                    let default = parts.next().map(|p| parse_expr(p)).transpose()?;
                    Ok(Some(TopLevel::Input(InputDecl {
                        type_name,
                        name,
                        default,
                        line,
                    })))
                }
                Rule::var_decl => {
                    let decl = parse_var_decl(inner, line)?;
                    Ok(Some(TopLevel::GlobalVar(decl)))
                }
                _ => Ok(None),
            }
        }
        Rule::function_def => {
            let func = parse_function_def(pair)?;
            Ok(Some(TopLevel::Function(func)))
        }
        Rule::enum_def => {
            let mut inner = pair.into_inner();
            let name = next_or_err(&mut inner, "enum_def name")?
                .as_str()
                .to_string();
            let mut members = Vec::new();
            for member in inner {
                if member.as_rule() == Rule::enum_member {
                    let mut parts = member.into_inner();
                    let mname = next_or_err(&mut parts, "enum_member name")?
                        .as_str()
                        .to_string();
                    let value = parts.next().map(|p| parse_expr(p)).transpose()?;
                    members.push((mname, value));
                }
            }
            Ok(Some(TopLevel::Enum(EnumDef {
                name,
                members,
                line,
            })))
        }
        Rule::struct_def => {
            let mut inner = pair.into_inner();
            let name = next_or_err(&mut inner, "struct_def name")?
                .as_str()
                .to_string();
            let mut fields = Vec::new();
            let mut methods = Vec::new();
            for item in inner {
                match item.as_rule() {
                    Rule::struct_field => {
                        let decl = parse_var_decl(item, line)?;
                        fields.push(decl);
                    }
                    Rule::function_def => {
                        methods.push(parse_function_def(item)?);
                    }
                    _ => {}
                }
            }
            Ok(Some(TopLevel::Struct(StructDef {
                name,
                fields,
                methods,
                line,
            })))
        }
        Rule::EOI => Ok(None),
        _ => Ok(None),
    }
}

fn parse_type_spec(pair: pest::iterators::Pair<'_, Rule>) -> String {
    pair.as_str().trim().to_string()
}

fn parse_var_decl(
    pair: pest::iterators::Pair<'_, Rule>,
    line: usize,
) -> Result<VarDecl, CompileError> {
    let text = pair.as_str();
    let is_static = text.starts_with("static");
    let is_const = text.contains("const");

    let mut inner = pair.into_inner();
    // Skip static/const keywords handled above
    let type_pair = next_or_err(&mut inner, "var_decl type")?;
    let type_name = parse_type_spec(type_pair);

    let var_init = next_or_err(&mut inner, "var_decl var_init")?;
    let mut var_parts = var_init.into_inner();
    let name = next_or_err(&mut var_parts, "var_decl name")?
        .as_str()
        .to_string();

    let mut is_array = false;
    let mut array_size = None;
    let mut init = None;

    for part in var_parts {
        match part.as_rule() {
            Rule::array_suffix => {
                is_array = true;
                if let Some(expr_pair) = part.into_inner().next() {
                    array_size = Some(parse_expr(expr_pair)?);
                }
            }
            Rule::array_init => {
                let elems: Vec<Expr> = part
                    .into_inner()
                    .map(|p| parse_expr(p))
                    .collect::<Result<_, _>>()?;
                init = Some(Expr::ArrayInit(elems));
            }
            _ => {
                init = Some(parse_expr(part)?);
            }
        }
    }

    Ok(VarDecl {
        type_name,
        name,
        is_static,
        is_const,
        is_array,
        array_size,
        init,
        line,
    })
}

fn parse_function_def(pair: pest::iterators::Pair<'_, Rule>) -> Result<FunctionDef, CompileError> {
    let line = pair.line_col().0;
    let text = pair.as_str();
    let is_static = text.starts_with("static");

    let mut inner = pair.into_inner();
    let return_type = parse_type_spec(next_or_err(&mut inner, "function_def return_type")?);
    let name = next_or_err(&mut inner, "function_def name")?
        .as_str()
        .to_string();

    let mut params = Vec::new();
    let mut body = Vec::new();

    for part in inner {
        match part.as_rule() {
            Rule::param_list => {
                for param_pair in part.into_inner() {
                    params.push(parse_param(param_pair)?);
                }
            }
            Rule::block => {
                body = parse_block(part)?;
            }
            _ => {}
        }
    }

    Ok(FunctionDef {
        return_type,
        name,
        params,
        body,
        is_static,
        line,
    })
}

fn parse_param(pair: pest::iterators::Pair<'_, Rule>) -> Result<Param, CompileError> {
    let text = pair.as_str();
    let is_ref = text.contains('&');

    let mut inner = pair.into_inner();
    let type_name = parse_type_spec(next_or_err(&mut inner, "param type")?);
    let name = next_or_err(&mut inner, "param name")?.as_str().to_string();

    let mut is_array = false;
    let mut default = None;

    for part in inner {
        match part.as_rule() {
            Rule::array_suffix => is_array = true,
            _ => default = Some(parse_expr(part)?),
        }
    }

    Ok(Param {
        type_name,
        name,
        is_ref,
        is_array,
        default,
    })
}

fn parse_block(pair: pest::iterators::Pair<'_, Rule>) -> Result<Vec<Stmt>, CompileError> {
    let mut stmts = Vec::new();
    for item in pair.into_inner() {
        stmts.push(parse_stmt(item)?);
    }
    Ok(stmts)
}

fn parse_stmt(pair: pest::iterators::Pair<'_, Rule>) -> Result<Stmt, CompileError> {
    let line = pair.line_col().0;
    match pair.as_rule() {
        Rule::stmt => {
            let inner = next_or_err(&mut pair.into_inner(), "stmt wrapper")?;
            return parse_stmt(inner);
        }
        Rule::var_decl => Ok(Stmt::VarDecl(parse_var_decl(pair, line)?)),
        Rule::expr_stmt => {
            let expr_pair = next_or_err(&mut pair.into_inner(), "expr_stmt")?;
            Ok(Stmt::Expr(parse_expr(expr_pair)?))
        }
        Rule::return_stmt => {
            let expr = pair
                .into_inner()
                .next()
                .map(|p| parse_expr(p))
                .transpose()?;
            Ok(Stmt::Return(expr))
        }
        Rule::if_stmt => {
            let mut inner = pair.into_inner();
            let cond = parse_expr(next_or_err(&mut inner, "if_stmt cond")?)?;
            let then = vec![parse_stmt(next_or_err(&mut inner, "if_stmt then")?)?];
            let else_ = match inner.next() {
                Some(p) => Some(vec![parse_stmt(p)?]),
                None => None,
            };
            Ok(Stmt::If {
                cond,
                then,
                else_,
                line,
            })
        }
        Rule::for_stmt => {
            let mut inner = pair.into_inner();
            let init_pair = next_or_err(&mut inner, "for_stmt init")?;
            let init = match init_pair.as_rule() {
                Rule::empty_stmt => None,
                _ => Some(Box::new(parse_stmt(init_pair)?)),
            };
            let cond = inner.next().map(|p| parse_expr(p)).transpose()?;
            // Skip semicolon
            let step = inner.next().map(|p| parse_expr(p)).transpose()?;
            let body_pair = next_or_err(&mut inner, "for_stmt body")?;
            let body = vec![parse_stmt(body_pair)?];
            Ok(Stmt::For {
                init,
                cond,
                step,
                body,
                line,
            })
        }
        Rule::while_stmt => {
            let mut inner = pair.into_inner();
            let cond = parse_expr(next_or_err(&mut inner, "while_stmt cond")?)?;
            let body = vec![parse_stmt(next_or_err(&mut inner, "while_stmt body")?)?];
            Ok(Stmt::While { cond, body, line })
        }
        Rule::do_while_stmt => {
            let mut inner = pair.into_inner();
            let body = vec![parse_stmt(next_or_err(&mut inner, "do_while body")?)?];
            let cond = parse_expr(next_or_err(&mut inner, "do_while cond")?)?;
            Ok(Stmt::DoWhile { body, cond, line })
        }
        Rule::break_stmt => Ok(Stmt::Break),
        Rule::continue_stmt => Ok(Stmt::Continue),
        Rule::block => Ok(Stmt::Block(parse_block(pair)?)),
        Rule::empty_stmt => Ok(Stmt::Empty),
        Rule::switch_stmt => {
            let mut inner = pair.into_inner();
            let expr = parse_expr(next_or_err(&mut inner, "switch_stmt expr")?)?;
            let mut cases = Vec::new();
            let mut default = None;
            for clause in inner {
                match clause.as_rule() {
                    Rule::case_clause => {
                        let mut parts = clause.into_inner();
                        let val = parse_expr(next_or_err(&mut parts, "case_clause value")?)?;
                        let stmts: Vec<Stmt> =
                            parts.map(|p| parse_stmt(p)).collect::<Result<_, _>>()?;
                        cases.push((val, stmts));
                    }
                    Rule::default_clause => {
                        let stmts: Vec<Stmt> = clause
                            .into_inner()
                            .map(|p| parse_stmt(p))
                            .collect::<Result<_, _>>()?;
                        default = Some(stmts);
                    }
                    _ => {}
                }
            }
            Ok(Stmt::Switch {
                expr,
                cases,
                default,
                line,
            })
        }
        _ => Ok(Stmt::Empty),
    }
}

fn parse_expr(pair: pest::iterators::Pair<'_, Rule>) -> Result<Expr, CompileError> {
    match pair.as_rule() {
        Rule::expr
        | Rule::assign_expr
        | Rule::ternary_expr
        | Rule::or_expr
        | Rule::and_expr
        | Rule::bitor_expr
        | Rule::xor_expr
        | Rule::bitand_expr
        | Rule::eq_expr
        | Rule::rel_expr
        | Rule::shift_expr
        | Rule::add_expr
        | Rule::mul_expr => {
            let mut inner = pair.into_inner();
            let first = next_or_err(&mut inner, "expr first operand")?;
            let mut result = parse_expr(first)?;

            while let Some(op_or_next) = inner.next() {
                // Could be an operator token or the next operand
                match op_or_next.as_rule() {
                    Rule::assign_op => {
                        let rhs = parse_expr(next_or_err(&mut inner, "assign rhs")?)?;
                        let op = match op_or_next.as_str() {
                            "=" => AssignOp::Assign,
                            "+=" => AssignOp::AddAssign,
                            "-=" => AssignOp::SubAssign,
                            "*=" => AssignOp::MulAssign,
                            "/=" => AssignOp::DivAssign,
                            "%=" => AssignOp::ModAssign,
                            "&=" => AssignOp::AndAssign,
                            "|=" => AssignOp::OrAssign,
                            "^=" => AssignOp::XorAssign,
                            "<<=" => AssignOp::ShlAssign,
                            ">>=" => AssignOp::ShrAssign,
                            _ => AssignOp::Assign,
                        };
                        result = Expr::Assign {
                            target: Box::new(result),
                            op,
                            value: Box::new(rhs),
                        };
                    }
                    _ => {
                        // Check if this is an operator token (plain string like "+", "-", "<", etc.)
                        let token_str = op_or_next.as_str();
                        let maybe_op = match token_str {
                            "+" => Some(BinOp::Add),
                            "-" => Some(BinOp::Sub),
                            "*" => Some(BinOp::Mul),
                            "/" => Some(BinOp::Div),
                            "%" => Some(BinOp::Mod),
                            "==" => Some(BinOp::Eq),
                            "!=" => Some(BinOp::Ne),
                            "<" => Some(BinOp::Lt),
                            "<=" => Some(BinOp::Le),
                            ">" => Some(BinOp::Gt),
                            ">=" => Some(BinOp::Ge),
                            "&&" => Some(BinOp::And),
                            "||" => Some(BinOp::Or),
                            "&" => Some(BinOp::BitAnd),
                            "|" => Some(BinOp::BitOr),
                            "^" => Some(BinOp::BitXor),
                            "<<" => Some(BinOp::Shl),
                            ">>" => Some(BinOp::Shr),
                            _ => None,
                        };
                        if let Some(op) = maybe_op {
                            // Next token is the right-hand operand
                            let rhs = parse_expr(next_or_err(&mut inner, "binop rhs")?)?;
                            result = Expr::BinOp {
                                op,
                                left: Box::new(result),
                                right: Box::new(rhs),
                            };
                        } else {
                            // Not an operator — treat as operand (fallback for nested expression rules)
                            let rhs = parse_expr(op_or_next)?;
                            result = Expr::BinOp {
                                op: BinOp::Add, // fallback for unknown structure
                                left: Box::new(result),
                                right: Box::new(rhs),
                            };
                        }
                    }
                }
            }
            Ok(result)
        }
        Rule::unary_expr => {
            let mut inner = pair.into_inner();
            let first = next_or_err(&mut inner, "unary_expr")?;
            if first.as_rule() == Rule::unary_op {
                let operand = parse_expr(next_or_err(&mut inner, "unary_expr operand")?)?;
                let op = match first.as_str() {
                    "-" => UnaryOp::Neg,
                    "!" => UnaryOp::Not,
                    "~" => UnaryOp::BitNot,
                    "++" => UnaryOp::PreIncr,
                    "--" => UnaryOp::PreDecr,
                    _ => UnaryOp::Neg,
                };
                Ok(Expr::UnaryOp {
                    op,
                    operand: Box::new(operand),
                })
            } else {
                parse_expr(first)
            }
        }
        Rule::postfix_expr => {
            let mut inner = pair.into_inner();
            let mut result = parse_expr(next_or_err(&mut inner, "postfix_expr base")?)?;
            for op in inner {
                // postfix_op wraps call_args/index_access/member_access or IS ++/--
                // For ++/--, the postfix_op has no inner children — it IS the operator
                let actual_op = if op.as_rule() == Rule::postfix_op {
                    let op_str = op.as_str();
                    if op_str == "++" || op_str == "--" {
                        op // ++/-- are leaf tokens, use directly
                    } else {
                        next_or_err(&mut op.into_inner(), "postfix_op inner")? // call_args/index_access/member_access
                    }
                } else {
                    op
                };
                match actual_op.as_rule() {
                    Rule::call_args => {
                        let func_name = match &result {
                            Expr::Ident(name) => name.clone(),
                            Expr::Member { field, .. } => field.clone(),
                            _ => "unknown".to_string(),
                        };
                        let args: Vec<Expr> = actual_op
                            .into_inner()
                            .map(|p| parse_expr(p))
                            .collect::<Result<_, _>>()?;
                        result = Expr::Call {
                            func: func_name,
                            args,
                        };
                    }
                    Rule::index_access => {
                        let idx = parse_expr(next_or_err(
                            &mut actual_op.into_inner(),
                            "index_access expr",
                        )?)?;
                        result = Expr::Index {
                            array: Box::new(result),
                            index: Box::new(idx),
                        };
                    }
                    Rule::member_access => {
                        let field =
                            next_or_err(&mut actual_op.into_inner(), "member_access field")?
                                .as_str()
                                .to_string();
                        result = Expr::Member {
                            object: Box::new(result),
                            field,
                        };
                    }
                    _ if actual_op.as_str() == "++" => {
                        result = Expr::PostIncr(Box::new(result));
                    }
                    _ if actual_op.as_str() == "--" => {
                        result = Expr::PostDecr(Box::new(result));
                    }
                    _ => {}
                }
            }
            Ok(result)
        }
        Rule::primary => {
            let inner = next_or_err(&mut pair.into_inner(), "primary")?;
            parse_expr(inner)
        }
        Rule::number_literal => {
            let s = pair.as_str();
            if s.contains('.') || s.contains('e') || s.contains('E') {
                Ok(Expr::FloatLit(s.parse::<f64>().unwrap_or(0.0)))
            } else if s.starts_with("0x") || s.starts_with("0X") {
                Ok(Expr::IntLit(i64::from_str_radix(&s[2..], 16).unwrap_or(0)))
            } else {
                Ok(Expr::IntLit(s.parse::<i64>().unwrap_or(0)))
            }
        }
        Rule::string_literal => {
            let s = pair.as_str();
            Ok(Expr::StringLit(s[1..s.len() - 1].to_string()))
        }
        Rule::bool_literal => Ok(Expr::BoolLit(pair.as_str() == "true")),
        Rule::null_literal => Ok(Expr::Null),
        Rule::color_literal => Ok(Expr::ColorLit(pair.as_str().to_string())),
        Rule::char_literal => {
            let s = pair.as_str();
            let ch = s.chars().nth(1).unwrap_or('\0');
            Ok(Expr::IntLit(ch as i64))
        }
        Rule::ident => Ok(Expr::Ident(pair.as_str().to_string())),
        _ => Ok(Expr::Null),
    }
}

#[cfg(test)]
mod tests;
