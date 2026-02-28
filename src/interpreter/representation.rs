use std::{fmt::Display, rc::Rc};

use crate::parser::syntax::BinaryOperator;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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

/// # Intermediate representation of a value
///
/// GetLocal = access the value of a local variable or function parameter
/// SetLocal = `let` block
/// Simple literal values
#[derive(Debug)]
pub enum IRExpr {
    GetLocal(LValue),
    SetLocal(LValue, Box<IRExpr>, Box<IRExpr>),
    Int(i64),
    Float(f64),
    String(Rc<str>),
    If {
        condition: Box<IRExpr>,
        body: Box<IRExpr>,
        else_body: Box<IRExpr>,
    },
    BinaryOperation(Box<IRExpr>, BinaryOperator, Box<IRExpr>),
}
