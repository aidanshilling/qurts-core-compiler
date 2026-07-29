use lower::cst_to_mlir::lower_program;
use melior::{
    Context,
    dialect::DialectRegistry,
    ir::operation::OperationLike,
    utility::register_all_dialects,
};
use parser::{QurtsParser, Rule};
use pest::Parser;
use std::fs;

enum Outcome {
    Lowers,
    Rejected,
}

fn test_context() -> Context {
    let context = Context::new();
    let registry = DialectRegistry::new();
    register_all_dialects(&registry);
    context.append_dialect_registry(&registry);
    context.load_all_available_dialects();
    qduc::dialect::register(&context);
    qauc::dialect::register(&context);
    context
}

/// Per-function expected outcome for each bundled example script, per the plan:
/// pass 1 only handles classical `func`/`arith`/`scf` constructs (plus flattened
/// tuples); anything needing `newlft`/`endlft`/`borrow_stmt` (qduc) or
/// `meas`/`unitary`/`lifted`/`qif` (qauc/gates) is rejected until later passes.
const EXPECTATIONS: &[(&str, &[(&str, Outcome)])] = &[
    ("basic.qurts", &[("always_true", Outcome::Lowers), ("choose", Outcome::Lowers)]),
    (
        "calls_and_tuples.qurts",
        &[
            ("pair", Outcome::Lowers),
            ("helper", Outcome::Lowers),
            ("combine", Outcome::Lowers),
            ("call_no_args", Outcome::Lowers),
            ("call_multi_args", Outcome::Lowers),
        ],
    ),
    (
        "lifetimes.qurts",
        &[
            ("borrow_example", Outcome::Rejected),
            ("scoped_lifetime", Outcome::Rejected),
            ("ordered_lifetimes", Outcome::Lowers),
        ],
    ),
    ("qif.qurts", &[("copy_via_qif", Outcome::Rejected)]),
    ("quantum_gates.qurts", &[("prepare_and_measure", Outcome::Rejected)]),
    ("uncompute_walkthrough.qurts", &[("example", Outcome::Rejected)]),
];

#[test]
fn example_scripts_lower_as_expected() {
    let scripts_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../parser/examples/scripts");

    for (file, expected_functions) in EXPECTATIONS {
        let path = format!("{scripts_dir}/{file}");
        let source =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));

        let context = test_context();
        let pairs = QurtsParser::parse(Rule::program, &source)
            .unwrap_or_else(|e| panic!("failed to parse {path}:\n{e}"));
        let result = lower_program(&context, pairs);

        println!("=== {file} ===");
        println!("{}", result.module.as_operation());
        for (name, error) in &result.errors {
            println!("  rejected {name}: {error}");
        }

        for (function_name, expected) in *expected_functions {
            let failed = result.errors.iter().any(|(name, _)| name == function_name);
            match expected {
                Outcome::Lowers => assert!(
                    !failed,
                    "{file}::{function_name} was expected to lower but was rejected: {:?}",
                    result.errors.iter().find(|(name, _)| name == function_name)
                ),
                Outcome::Rejected => assert!(
                    failed,
                    "{file}::{function_name} was expected to be rejected but lowered successfully"
                ),
            }
        }

        assert!(
            result.module.as_operation().verify(),
            "{file}: lowered module failed to verify"
        );
    }
}
