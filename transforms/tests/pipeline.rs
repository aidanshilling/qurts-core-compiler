use lower::lower_program;
use melior::ir::{Module, operation::OperationLike};
use parser::{QurtsParser, Rule};
use pest::Parser;
use std::fs;
use transforms::{CONTROLLED_CONJUGATION, INFER_DROPS, MQT_OPTIMIZE, TO_QCO, fidelity, nested_in, reparse, run_pipeline, simulate};

fn script(name: &str) -> String {
    let path = format!("{}/../parser/examples/scripts/{name}", env!("CARGO_MANIFEST_DIR"));
    fs::read_to_string(&path).unwrap_or_else(|e| panic!("failed to read {path}: {e}"))
}

fn lower_source<'c>(context: &'c melior::Context, source: &str) -> Module<'c> {
    let pairs = QurtsParser::parse(Rule::program, source).expect("source parses");
    let lowered = lower_program(context, pairs);
    assert!(lowered.errors.is_empty(), "lowering errors: {:?}", lowered.errors);
    lowered.module
}

fn to_qco<'c>(context: &'c melior::Context, source: &str) -> Module<'c> {
    let mut module = lower_source(context, source);
    run_pipeline(context, &mut module, INFER_DROPS).expect("drop inference");
    run_pipeline(context, &mut module, TO_QCO).expect("qco conversion");
    assert!(module.as_operation().verify());
    module
}

fn assert_equivalent(context: &melior::Context, a: &Module, b: &Module) {
    let sa = simulate(context, a, "main").expect("naive simulates");
    let sb = simulate(context, b, "main").expect("optimized simulates");
    let f = fidelity(&sa, &sb);
    assert!((f - 1.0).abs() < 1e-9, "fidelity {f}\n--- a ---\n{}\n--- b ---\n{}", a.as_operation(), b.as_operation());
}

#[test]
fn ancilla_dropped_inside_qif_is_conjugated_out_of_the_control() {
    let context = transforms::context();
    let naive = to_qco(&context, &script("qif_ancilla.qurts"));
    let mut optimized = reparse(&context, &naive);
    run_pipeline(&context, &mut optimized, CONTROLLED_CONJUGATION).expect("conjugation");
    assert!(nested_in(&naive, "qco.ctrl", "qco.inv"), "{}", naive.as_operation());
    assert!(!nested_in(&optimized, "qco.ctrl", "qco.inv"), "{}", optimized.as_operation());
    assert_equivalent(&context, &naive, &optimized);

    let mut mqt = reparse(&context, &optimized);
    run_pipeline(&context, &mut mqt, MQT_OPTIMIZE).expect("mqt pipeline");
    assert_equivalent(&context, &naive, &mqt);
}

#[test]
fn nested_qif_with_else_branch_stays_equivalent_through_every_stage() {
    let context = transforms::context();
    let naive = to_qco(&context, &script("nested_qif.qurts"));
    let mut optimized = reparse(&context, &naive);
    run_pipeline(&context, &mut optimized, CONTROLLED_CONJUGATION).expect("conjugation");
    run_pipeline(&context, &mut optimized, MQT_OPTIMIZE).expect("mqt pipeline");
    assert!(!nested_in(&optimized, "qco.ctrl", "qco.inv"), "{}", optimized.as_operation());
    assert_equivalent(&context, &naive, &optimized);
}

#[test]
fn qif_result_dropped_before_control_lifetime_ends_uncomputes_via_control() {
    let context = transforms::context();
    let module = to_qco(&context, &script("toy_example.qurts"));
    let text = module.as_operation().to_string();
    assert!(text.contains("qco.inv"), "{text}");
    let state = simulate(&context, &module, "main").expect("simulates");
    let mass: f64 = state.iter().map(|(re, im)| re * re + im * im).sum();
    assert!((mass - 1.0).abs() < 1e-9);
    assert!(state.iter().filter(|(re, im)| re.abs() + im.abs() > 1e-9).count() == 2, "{text}\n{state:?}");
}

#[test]
fn unconsumed_unitary_result_is_a_drop_inference_error() {
    let context = transforms::context();
    let mut module = lower_source(&context, "fn main() -> bool { let q : qbit = [0](); let h : qbit = H(q); true }");
    let error = run_pipeline(&context, &mut module, INFER_DROPS).unwrap_err();
    assert!(error.contains("linear qubit is never consumed"), "{error}");
}

#[test]
fn qif_yielding_different_outer_qubits_is_rejected() {
    let context = transforms::context();
    let source = "fn main() -> bool {
        let a : qbit = [0](); let b : qbit = [0]();
        let c0 : qbit = [0](); let c : qbit = H(c0);
        newlft 'a; let r = &'a c;
        let y : qbit = qif r { a } else { b };
        endlft 'a;
        meas(y)
    }";
    let mut module = lower_source(&context, source);
    let error = run_pipeline(&context, &mut module, TO_QCO).unwrap_err();
    assert!(error.contains("different qubits"), "{error}");
}

#[test]
fn dropping_an_outer_qubit_inside_a_branch_is_rejected() {
    let context = transforms::context();
    let source = "fn main() -> bool {
        let a : qbit = [0]();
        let c0 : qbit = [0](); let c : qbit = H(c0);
        newlft 'a; let r = &'a c;
        let y : qbit = qif r { drop a; [0]() } else { drop a; [0]() };
        endlft 'a;
        meas(y)
    }";
    let mut module = lower_source(&context, source);
    let error = run_pipeline(&context, &mut module, TO_QCO).unwrap_err();
    assert!(error.contains("enclosing qif branch"), "{error}");
}
