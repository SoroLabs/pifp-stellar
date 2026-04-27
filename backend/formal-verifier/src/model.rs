use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ValueSort {
    Int,
    Bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IntExpr {
    Const(i128),
    Var(String),
    Add(Box<IntExpr>, Box<IntExpr>),
    Sub(Box<IntExpr>, Box<IntExpr>),
    Mul(Box<IntExpr>, Box<IntExpr>),
    Div(Box<IntExpr>, Box<IntExpr>),
    Rem(Box<IntExpr>, Box<IntExpr>),
    Ite(Box<BoolExpr>, Box<IntExpr>, Box<IntExpr>),
    Fresh(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoolExpr {
    Const(bool),
    Var(String),
    Not(Box<BoolExpr>),
    And(Vec<BoolExpr>),
    Or(Vec<BoolExpr>),
    Eq(Box<IntExpr>, Box<IntExpr>),
    Ne(Box<IntExpr>, Box<IntExpr>),
    Lt(Box<IntExpr>, Box<IntExpr>),
    Le(Box<IntExpr>, Box<IntExpr>),
    Gt(Box<IntExpr>, Box<IntExpr>),
    Ge(Box<IntExpr>, Box<IntExpr>),
    Ite(Box<BoolExpr>, Box<BoolExpr>, Box<BoolExpr>),
    Fresh(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueExpr {
    Int(IntExpr),
    Bool(BoolExpr),
}

#[derive(Debug, Clone)]
pub struct PathState {
    pub stack: Vec<ValueExpr>,
    pub locals: BTreeMap<u32, ValueExpr>,
    pub globals: BTreeMap<u32, ValueExpr>,
    pub path_condition: BoolExpr,
    pub notes: Vec<String>,
}

impl PathState {
    pub fn new() -> Self {
        Self {
            stack: Vec::new(),
            locals: BTreeMap::new(),
            globals: BTreeMap::new(),
            path_condition: BoolExpr::Const(true),
            notes: Vec::new(),
        }
    }

    pub fn push(&mut self, value: ValueExpr) {
        self.stack.push(value);
    }

    pub fn pop(&mut self) -> Option<ValueExpr> {
        self.stack.pop()
    }

    pub fn require_int(value: ValueExpr) -> Result<IntExpr, String> {
        match value {
            ValueExpr::Int(expr) => Ok(expr),
            ValueExpr::Bool(_) => Err("expected integer value on stack".to_string()),
        }
    }

    pub fn require_bool(value: ValueExpr) -> Result<BoolExpr, String> {
        match value {
            ValueExpr::Bool(expr) => Ok(expr),
            ValueExpr::Int(_) => Err("expected boolean value on stack".to_string()),
        }
    }
}

impl Default for PathState {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub enum Node {
    Op(Op),
    Block(Vec<Node>),
    Loop(Vec<Node>),
    If {
        then_nodes: Vec<Node>,
        else_nodes: Vec<Node>,
    },
}

#[derive(Debug, Clone)]
pub enum Op {
    Unreachable,
    Nop,
    Drop,
    Select,
    Return,
    LocalGet(u32),
    LocalSet(u32),
    LocalTee(u32),
    GlobalGet(u32),
    GlobalSet(u32),
    I32Const(i32),
    I64Const(i64),
    I32Eqz,
    I64Eqz,
    I32Add,
    I32Sub,
    I32Mul,
    I32DivS,
    I32DivU,
    I32RemS,
    I32RemU,
    I32And,
    I32Or,
    I32Xor,
    I32Shl,
    I32ShrS,
    I32ShrU,
    I64Add,
    I64Sub,
    I64Mul,
    I64DivS,
    I64DivU,
    I64RemS,
    I64RemU,
    I64And,
    I64Or,
    I64Xor,
    I64Shl,
    I64ShrS,
    I64ShrU,
    EqI32,
    EqI64,
    NeI32,
    NeI64,
    LtSI32,
    LtSI64,
    LeSI32,
    LeSI64,
    GtSI32,
    GtSI64,
    GeSI32,
    GeSI64,
    Block,
    Loop,
    If,
    Else,
    End,
    Br(u32),
    BrIf(u32),
    Call(u32),
    CallIndirect,
    LoadFreshInt(String),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub nodes: Vec<Node>,
}

#[derive(Debug, Clone)]
pub struct FunctionSummary {
    pub index: u32,
    pub export_name: Option<String>,
    pub program: Program,
    pub op_count: usize,
    pub unsupported_ops: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct ModuleSummary {
    pub functions: Vec<FunctionSummary>,
}
