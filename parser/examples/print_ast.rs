use parser::{QurtsParser, Rule, format_parse_tree};
use pest::Parser;
use std::{env, fs, process};

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: print_ast <file.qurts>");
        process::exit(1);
    };
    let source = fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));

    match QurtsParser::parse(Rule::program, &source) {
        Ok(pairs) => print!("{}", format_parse_tree(pairs)),
        Err(e) => {
            eprintln!("parse error in {path}:\n{e}");
            process::exit(1);
        }
    }
}
