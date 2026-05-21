mod ast;
mod emitter;
mod lexer;
mod parser;

use std::path::PathBuf;
use std::fs;
use clap::Parser as ClapParser;
use crate::lexer::Lexer;
use crate::parser::Parser;

#[derive(ClapParser)]
#[command(about = "Convert ABC notation to LilyPond")]
struct Args {
    /// Input ABC file
    input: PathBuf,

    /// Output LilyPond file (default: input with .ly extension)
    #[arg(short, long)]
    output: Option<PathBuf>,
}

fn main() {
    let args = Args::parse();

    let input = match fs::read_to_string(&args.input) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error reading {}: {e}", args.input.display());
            std::process::exit(1);
        }
    };

    let output_path = args.output.unwrap_or_else(|| args.input.with_extension("ly"));

    let ly = match Parser::new(Lexer::new(&input)).parse() {
        Ok(tune) => emitter::emit(&tune),
        Err(e) => {
            eprintln!("parse error: {e:?}");
            std::process::exit(1);
        }
    };

    if let Err(e) = fs::write(&output_path, &ly) {
        eprintln!("error writing {}: {e}", output_path.display());
        std::process::exit(1);
    }

    println!("wrote {}", output_path.display());
}
