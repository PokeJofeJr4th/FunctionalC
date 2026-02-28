use crate::interpreter::representation::IRExpr;

pub mod c;

pub trait Compiler {
    type Output;

    fn compile(&self, expr: &IRExpr) -> Self::Output;
}
