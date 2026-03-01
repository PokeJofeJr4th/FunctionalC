use std::rc::Rc;

#[derive(Debug)]
pub enum Expression {
    String(Rc<str>),
    Ident(Rc<str>),
    Int(i64),
    Float(f64),
    BinaryOperation(Box<Expression>, BinaryOperator, Box<Expression>),
    If {
        condition: Box<Expression>,
        body: Box<Expression>,
        else_body: Box<Expression>,
    },
    Let {
        var: Rc<str>,
        val: Box<Expression>,
        body: Box<Expression>,
    },
    FunctionCall {
        function: Box<Expression>,
        args: Vec<Expression>,
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
}
