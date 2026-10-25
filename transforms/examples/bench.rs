use lower::lower_program;
use parser::{QurtsParser, Rule};
use pest::Parser;
use std::fs;
use transforms::{CONTROLLED_CONJUGATION, INFER_DROPS, MQT_OPTIMIZE, TO_QCO, fidelity, gate_summary, reparse, run_pipeline, simulate};

fn main() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../parser/examples/scripts");
    let mut paths: Vec<_> = fs::read_dir(dir).unwrap().filter_map(Result::ok).map(|e| e.path()).collect();
    paths.sort();
    for path in paths.iter().filter(|p| p.extension().is_some_and(|e| e == "qurts")) {
        let source = fs::read_to_string(path).unwrap();
        if !source.contains("fn main") {
            continue;
        }
        println!("=== {} ===", path.file_name().unwrap().to_string_lossy());
        let context = transforms::context();
        let pairs = QurtsParser::parse(Rule::program, &source).unwrap();
        let mut module = lower_program(&context, pairs).module;
        if let Err(error) = run_pipeline(&context, &mut module, INFER_DROPS)
            .and_then(|()| run_pipeline(&context, &mut module, TO_QCO))
        {
            println!("  {error}\n");
            continue;
        }
        let mut naive = reparse(&context, &module);
        let mut conjugated = reparse(&context, &module);
        let steps = [(&mut naive, vec![MQT_OPTIMIZE]), (&mut conjugated, vec![CONTROLLED_CONJUGATION, MQT_OPTIMIZE])];
        for (target, pipelines) in steps {
            for pipeline in pipelines {
                if let Err(error) = run_pipeline(&context, target, pipeline) {
                    println!("  {error}");
                }
            }
        }
        println!("--- naive + mqt-qco-default ---\n{}", gate_summary(&naive));
        println!("--- controlled-conjugation + mqt-qco-default ---\n{}", gate_summary(&conjugated));
        match (simulate(&context, &module, "main"), simulate(&context, &conjugated, "main")) {
            (Ok(a), Ok(b)) => println!("fidelity(unoptimized, conjugated+mqt) = {:.12}", fidelity(&a, &b)),
            (a, b) => println!("simulation unavailable: {a:?} {b:?}"),
        }
        println!();
    }
}
