use parser::{QurtsParser, Rule, format_parse_tree};
use pest::Parser;
use std::fs;

#[test]
fn example_scripts_parse_and_print() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/examples/scripts");
    let mut scripts: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("failed to read {dir}: {e}"))
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.extension().is_some_and(|ext| ext == "qurts"))
        .collect();
    scripts.sort();
    assert!(!scripts.is_empty(), "no .qurts example scripts found in {dir}");

    for path in scripts {
        let source = fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path:?}: {e}"));
        let pairs = QurtsParser::parse(Rule::program, &source)
            .unwrap_or_else(|e| panic!("failed to parse {path:?}:\n{e}"));

        println!("=== {} ===", path.display());
        println!("{}", format_parse_tree(pairs));
    }
}
