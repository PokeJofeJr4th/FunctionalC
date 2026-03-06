#![warn(clippy::pedantic, clippy::nursery)]
use std::{path::PathBuf, process::ExitCode};

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

fn main() -> ExitCode {
    let args = Args::parse();

    let src = match std::fs::read_to_string(args.src) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let syn = match parser::grammar::ProgramParser::new().parse(&src) {
        Ok(e) => e,
        Err(e) => {
            eprintln!("Error: {e}");
            return ExitCode::FAILURE;
        }
    };

    // println!("{syn:#?}");

    let ir = match interpreter::interpret(syn) {
        Ok(ir) => ir,
        Err(err) => {
            eprintln!("Error: {err}");
            return ExitCode::FAILURE;
        }
    };

    // println!("{ir:#?}");

    let compiled = match CCompiler::new().compile(&ir) {
        Ok(c) => c,
        Err(err) => {
            eprintln!("Error: {err}");
            return ExitCode::FAILURE;
        }
    };

    if let Err(err) = std::fs::write(args.destination, compiled) {
        eprintln!("Error: {err}");
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}
