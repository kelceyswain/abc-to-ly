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

    /// LilyPond style file to include after \\version (e.g. \\paper block, staff size)
    #[arg(short, long)]
    style: Option<PathBuf>,

    /// Run lilypond to produce a PDF
    #[arg(short, long)]
    compile: bool,

    /// Run lilypond to produce an SVG (implies --compile)
    #[arg(long)]
    svg: bool,
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

    let style = args.style.as_ref().map(|p| fs::read_to_string(p).unwrap_or_else(|e| {
        eprintln!("error reading style {}: {e}", p.display());
        std::process::exit(1);
    }));

    let output_path = args.output.unwrap_or_else(|| args.input.with_extension("ly"));

    let ly = match Parser::new(Lexer::new(&input)).parse() {
        Ok(tune) => emitter::emit(&tune, style.as_deref()),
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

    if args.compile || args.svg {
        run_lilypond(&output_path, args.svg);
    }
}

fn run_lilypond(ly_path: &PathBuf, svg: bool) {
    let mut cmd = std::process::Command::new("lilypond");

    if svg {
        cmd.arg("--svg");
        cmd.arg("-dno-point-and-click");
        cmd.arg("-dno-use-paper-size-for-page");
    }

    // Place output next to the .ly file rather than in the current directory
    cmd.arg("-o").arg(ly_path.with_extension(""));
    cmd.arg(ly_path);

    match cmd.status() {
        Ok(status) if status.success() => {}
        Ok(status) => {
            eprintln!("lilypond exited with {status}");
            std::process::exit(status.code().unwrap_or(1));
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            eprintln!("lilypond not found — install it to compile .ly files");
        }
        Err(e) => {
            eprintln!("error running lilypond: {e}");
            std::process::exit(1);
        }
    }
}
