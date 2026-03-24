//! MQL5 Abstract Syntax Tree.
//!
//! Represents the full structure of an MQL5 program after parsing.

use serde::Serialize;

/// A complete MQL5 program (indicator, EA, or script).
#[derive(Debug, Clone, Serialize)]
pub struct Program {
    pub items: Vec<TopLevel>,
}

#[derive(Debug, Clone, Serialize)]
pub enum TopLevel {
    Property(Property),
    Include(String),
    Define(String, Option<Expr>),
    Input(InputDecl),
    GlobalVar(VarDecl),
    Function(FunctionDef),
    Enum(EnumDef),
    Struct(StructDef),
}

/// #property directive
#[derive(Debug, Clone, Serialize)]
pub struct Property {
    pub name: String,
    pub value: Expr,
    pub line: usize,
}

/// input variable declaration
#[derive(Debug, Clone, Serialize)]
pub struct InputDecl {
    pub type_name: String,
    pub name: String,
    pub default: Option<Expr>,
    pub line: usize,
}

/// Variable declaration
#[derive(Debug, Clone, Serialize)]
pub struct VarDecl {
    pub type_name: String,
    pub name: String,
    pub is_static: bool,
    pub is_const: bool,
    pub is_array: bool,
    pub array_size: Option<Expr>,
    pub init: Option<Expr>,
    pub line: usize,
}

/// Function definition
#[derive(Debug, Clone, Serialize)]
pub struct FunctionDef {
    pub return_type: String,
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub is_static: bool,
    pub line: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct Param {
    pub type_name: String,
    pub name: String,
    pub is_ref: bool,
    pub is_array: bool,
    pub default: Option<Expr>,
}

/// Enum definition
#[derive(Debug, Clone, Serialize)]
pub struct EnumDef {
    pub name: String,
    pub members: Vec<(String, Option<Expr>)>,
    pub line: usize,
}

/// Struct definition
#[derive(Debug, Clone, Serialize)]
pub struct StructDef {
    pub name: String,
    pub fields: Vec<VarDecl>,
    pub methods: Vec<FunctionDef>,
    pub line: usize,
}

/// Statement
#[derive(Debug, Clone, Serialize)]
pub enum Stmt {
    VarDecl(VarDecl),
    Expr(Expr),
    Return(Option<Expr>),
    If { cond: Expr, then: Vec<Stmt>, else_: Option<Vec<Stmt>>, line: usize },
    For { init: Option<Box<Stmt>>, cond: Option<Expr>, step: Option<Expr>, body: Vec<Stmt>, line: usize },
    While { cond: Expr, body: Vec<Stmt>, line: usize },
    DoWhile { body: Vec<Stmt>, cond: Expr, line: usize },
    Switch { expr: Expr, cases: Vec<(Expr, Vec<Stmt>)>, default: Option<Vec<Stmt>>, line: usize },
    Break,
    Continue,
    Block(Vec<Stmt>),
    Empty,
}

/// Expression
#[derive(Debug, Clone, Serialize)]
pub enum Expr {
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),
    BoolLit(bool),
    ColorLit(String),
    Null,
    Ident(String),
    BinOp { op: BinOp, left: Box<Expr>, right: Box<Expr> },
    UnaryOp { op: UnaryOp, operand: Box<Expr> },
    Assign { target: Box<Expr>, op: AssignOp, value: Box<Expr> },
    Call { func: String, args: Vec<Expr> },
    Index { array: Box<Expr>, index: Box<Expr> },
    Member { object: Box<Expr>, field: String },
    Ternary { cond: Box<Expr>, then: Box<Expr>, else_: Box<Expr> },
    Cast { target_type: String, expr: Box<Expr> },
    PostIncr(Box<Expr>),
    PostDecr(Box<Expr>),
    ArrayInit(Vec<Expr>),
}

#[derive(Debug, Clone, Serialize)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod,
    Eq, Ne, Lt, Le, Gt, Ge,
    And, Or,
    BitAnd, BitOr, BitXor, Shl, Shr,
}

#[derive(Debug, Clone, Serialize)]
pub enum UnaryOp {
    Neg, Not, BitNot, PreIncr, PreDecr,
}

#[derive(Debug, Clone, Serialize)]
pub enum AssignOp {
    Assign, AddAssign, SubAssign, MulAssign, DivAssign, ModAssign,
    AndAssign, OrAssign, XorAssign, ShlAssign, ShrAssign,
}
