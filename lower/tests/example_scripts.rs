use lower::{default_context, lower_program};
use melior::ir::operation::OperationLike;
use parser::{QurtsParser, Rule};
use pest::Parser;
use std::fs;

enum Outcome {
    Lowers,
    Rejected,
}

/// Per-function expected outcome for each bundled example script, per the plan:
/// classical `func`/`arith`/`scf` constructs (plus flattened tuples) and
/// `newlft`/`endlft` (qduc) lower; `borrow_stmt` (qduc, needs a qauc.borrow op —
/// separate follow-up) and `meas`/`unitary`/`lifted`/`qif` (qauc/gates) are
/// still rejected.
const EXPECTATIONS: &[(&str, &[(&str, Outcome)])] = &[
    (
        "basic.qurts",
        &[
            ("always_true", Outcome::Lowers),
            ("choose", Outcome::Lowers),
        ],
    ),
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
            ("scoped_lifetime", Outcome::Lowers),
            ("ordered_lifetimes", Outcome::Lowers),
        ],
    ),
    ("qif.qurts", &[("copy_via_qif", Outcome::Rejected)]),
    (
        "quantum_gates.qurts",
        &[("prepare_and_measure", Outcome::Rejected)],
    ),
    (
        "uncompute_walkthrough.qurts",
        &[("example", Outcome::Rejected)],
    ),
];

#[test]
fn example_scripts_lower_as_expected() {
    let scripts_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../parser/examples/scripts");

    for (file, expected_functions) in EXPECTATIONS {
        let path = format!("{scripts_dir}/{file}");
        let source =
            fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"));

        let context = default_context();
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
