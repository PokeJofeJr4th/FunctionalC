use crate::interpreter::representation::IRValue;

pub mod c;

pub trait Compiler {
    type Output;
    type Error;

    fn compile(self, expr: &IRValue) -> Result<Self::Output, Self::Error>;
}
