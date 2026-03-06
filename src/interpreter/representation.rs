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
        write!(f, "{self}")
    }
}

#[derive(Debug)]
pub struct IRValue(IRExpr, IRType);

impl IRValue {
    pub const fn type_hint(&self) -> &IRType {
        &self.1
    }

    pub const fn expr(&self) -> &IRExpr {
        &self.0
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub enum Builtin {
    /// generic<T> (value: T) -> IO<T>
    ///
    /// Just returns that value from the monad
    Return,
    /// IO<str>
    ///
    /// Reads one line from the terminal
    ReadLine,
    /// (text: str) -> IO<()>
    ///
    /// Prints one line of text to the terminal
    WriteLine,
}

impl Display for Builtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Return => write!(f, "return"),
            Self::ReadLine => write!(f, "readLine"),
            Self::WriteLine => write!(f, "writeLine"),
        }
    }
}

impl Debug for Builtin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self}")
    }
}

/// # Intermediate representation of a value
///
/// `GetLocal` = access the value of a local variable or function parameter
/// `SetLocal` = `let` block
/// Simple literal values
#[derive(Debug)]
pub enum IRExpr {
    GetLocal(LValue),
    SetLocal(LValue, Box<IRValue>, Box<IRValue>),
    BindIoMonad {
        var_name: LValue,
        var_value: Box<IRValue>,
        body: Box<IRValue>,
        captures: Vec<(LValue, IRType)>,
    },
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
    Builtin(Builtin),
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
    IOMonad(Option<Box<Self>>),
    Function {
        inputs: Vec<Self>,
        output: Box<Self>,
    },
}

impl IRType {
    pub const fn is_function(&self) -> bool {
        matches!(self, Self::Function { .. })
    }

    pub const fn is_io_monad(&self) -> bool {
        matches!(self, Self::IOMonad(..))
    }
}
