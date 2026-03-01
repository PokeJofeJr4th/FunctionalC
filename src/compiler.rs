use crate::interpreter::representation::IRValue;

pub mod c;

pub trait Compiler {
    type Output;

    fn compile(self, expr: &IRValue) -> Self::Output;
}
