mod ast;
mod emitter;
mod lexer;
mod parser;

use crate::lexer::Lexer;
use crate::parser::Parser;

fn main() {
    // let input = "T:The Morning Dew\nM:6/8\nL:1/8\nK:D\nabc ABC | def DEF";
    let input = "X: 1\nT: The Connaughtman's Rambles\nR: jig\nM: 6/8\nL: 1/8\nK: Dmaj\n|:FAA dAA|BAA dAG|FAA dfe|dBB BAG|\nFAA dAA|BAA def|gfe dfe|1 dBB BAG:|2 dBB B3||\n|:fbb faf|fed ede|fbb faf|fed e3|\nfbb faf|fed def|gfe dfe|1 dBB B3:|2 dBB BAG||";

    match Parser::new(Lexer::new(input)).parse() {
        Ok(tune) => print!("{}", emitter::emit(&tune)),
        Err(e)   => eprintln!("parse error: {e:?}"),
    }
}
