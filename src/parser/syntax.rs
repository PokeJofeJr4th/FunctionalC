use std::rc::Rc;

use crate::interpreter::representation::IRType;

#[derive(Debug)]
pub enum Expression {
    String(Rc<str>),
    Ident(Rc<str>),
    Int(i64),
    Float(f64),
    BinaryOperation(Box<Self>, BinaryOperator, Box<Self>),
    If {
        condition: Box<Self>,
        body: Box<Self>,
        else_body: Box<Self>,
    },
    Let {
        var: Rc<str>,
        val: Box<Self>,
        body: Box<Self>,
    },
    FunctionCall {
        function: Box<Self>,
        args: Vec<Self>,
    },
    Function {
        args: Vec<(Rc<str>, IRType)>,
        body: Box<Self>,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Sub,
    Div,
    Mul,
    Mod,
    Eq,
    Ne,
    Le,
    Ge,
    Lt,
    Gt,
    Dot,
    Index,
    And,
    Or,
}
