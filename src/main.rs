#![warn(clippy::pedantic, clippy::nursery)]
use std::path::PathBuf;

use clap::Parser;

use crate::compiler::{Compiler, c::CCompiler};

mod compiler;
mod interpreter;
mod parser;

#[derive(Parser)]
struct Args {
    src: PathBuf,
    destination: PathBuf,
}

fn main() {
    let args = Args::parse();

    let src = std::fs::read_to_string(args.src).unwrap();

    let syn = parser::grammar::ProgramParser::new().parse(&src).unwrap();
    // println!("{syn:#?}");
    match interpreter::interpret(syn) {
        Ok(res) => {
            // println!("{res:#?}");
            let c = CCompiler::new().compile(&res).unwrap();
            std::fs::write(args.destination, c).unwrap();
        }
        Err(err) => {
            println!("Error: {err}");
        }
    }
}
