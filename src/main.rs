use crate::compiler::{Compiler, c::CCompiler};

mod interpreter;
mod parser;
mod compiler;

fn main() {
    let syn = parser::grammar::ProgramParser::new()
        .parse(include_str!("../test.fc"))
        .unwrap();
    // println!("{syn:#?}");
    match interpreter::interpret(syn) {
        Ok(res) => {
            // println!("{res:#?}");
            let c = CCompiler::new().compile(&res).unwrap();
            println!("{c}");
        }
        Err(err) => {
            println!("Error: {err}")
        }
    }
}
