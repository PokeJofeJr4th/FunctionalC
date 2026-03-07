use std::rc::Rc;

#[derive(Debug)]
pub enum Expression {
    /// String literal
    String(Rc<str>),
    /// Identifier
    Ident(Rc<str>),
    /// Integer
    Int(i64),
    ///  Floating-Point Number
    Float(f64),
    /// Two expressions with some kind of addition/multiplication/etc between them
    BinaryOperation(Box<Self>, BinaryOperator, Box<Self>),
    /// Expression of the form `<condition> ? <body> : <else_body>`
    Ternary {
        condition: Box<Self>,
        body: Box<Self>,
        else_body: Box<Self>,
    },
    /// Variable binding: `let <var> = <val>; <body>`
    Let {
        var: Rc<str>,
        val: Box<Self>,
        body: Box<Self>,
    },
    /// Monadic unwrapping binding: `let <var> := <val>; <body>`
    ///
    /// `<val>` and `<body>` must be expressions that represent IO monads, and `<val>` must not be an `IO<void>`. Within `<body>`, `<var>` can be used as the inner type of `<val>` (e.g. if `<val>` is an `IO<str>`, then `<var>` can be treated as a `str`)
    MonadLet {
        var: Rc<str>,
        val: Box<Self>,
        body: Box<Self>,
    },
    /// A block that defines a type alias
    TypeAlias {
        alias: Rc<str>,
        typ: TypeExpr,
        body: Box<Self>
    },
    /// Two `IO<()>` monads composed together: `<first>; <second>`
    ComposeMonads(Box<Self>, Box<Self>),
    /// Invoking a function: `<function>(<args>, ..)`
    FunctionCall {
        function: Box<Self>,
        args: Vec<Self>,
    },
    /// Creating a function: `(<args>, ...) => <body>`
    Function {
        args: Vec<(Rc<str>, TypeExpr)>,
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

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum TypeExpr {
    Int,
    Float,
    String,
    Boolean,
    IOMonad(Option<Box<Self>>),
    Function {
        inputs: Vec<Self>,
        output: Box<Self>,
    },
    Named(Rc<str>),
}
