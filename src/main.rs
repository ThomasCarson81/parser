use std::env;
use syntax::Token;

enum ParserError {
    UnexpectedToken(Token),
    UnexpectedEOF,
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 && args.len() != 3 {
        println!(
            "Please provide a file to parse, and optionally an output file to write the AST to."
        );
        return;
    }

    if args.len() == 3 {
        // output result to file
    } else {
        // print result to console
    }
}
