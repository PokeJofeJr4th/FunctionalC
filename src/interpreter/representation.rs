use std::{
    fmt::{Debug, Display},
    rc::Rc,
};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct LValue(usize);

static mut NEXT_LVALUE: usize = 0;

impl LValue {
    pub const fn new() -> Self {
        Self(unsafe {
            NEXT_LVALUE += 1;
            NEXT_LVALUE
        })
    }
}

impl Display for LValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "_{}", self.0)
    }
}

impl Debug for LValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

#[derive(Debug)]
pub struct IRValue(pub IRExpr, pub IRType);

/// # Intermediate representation of a value
///
/// GetLocal = access the value of a local variable or function parameter
/// SetLocal = `let` block
/// Simple literal values
#[derive(Debug)]
pub enum IRExpr {
    GetLocal(LValue),
    SetLocal(LValue, Box<IRValue>, Box<IRValue>),
    Int(i64),
    Float(f64),
    String(Rc<str>),
    If {
        condition: Box<IRValue>,
        body: Box<IRValue>,
        else_body: Box<IRValue>,
    },
    Arithmetic(Box<IRValue>, ArithmeticOperator, Box<IRValue>),
    Comparison(Box<IRValue>, ComparisonOperator, Box<IRValue>),
    Boolean(Box<IRValue>, BooleanOperator, Box<IRValue>),
    Function {
        params: Vec<LValue>,
        captures: Vec<(LValue, IRType)>,
        body: Box<IRValue>,
    },
    FunctionCall(Box<IRValue>, Vec<IRValue>),
}

impl IRExpr {
    pub const fn typed(self, ty: IRType) -> IRValue {
        IRValue(self, ty)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArithmeticOperator {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComparisonOperator {
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BooleanOperator {
    And,
    Or,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum IRType {
    Int,
    Float,
    String,
    Boolean,
    Function {
        inputs: Vec<IRType>,
        output: Box<IRType>,
    },
}
