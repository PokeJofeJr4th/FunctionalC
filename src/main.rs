use crate::compiler::{Compiler, c::CCompiler};

mod interpreter;
mod parser;
mod compiler;

fn main() {
    let syn = parser::grammar::ProgramParser::new()
        .parse("let x = 11.0; x == 0 ? 21 : x + x")
        .unwrap();
    println!("{syn:?}");
    match interpreter::interpret(syn) {
        Ok(res) => {
            println!("{res:?}");
            let c = CCompiler.compile(&res);
            println!("{c}");
        }
        Err(err) => {
            println!("Error: {err}")
        }
    }
}
