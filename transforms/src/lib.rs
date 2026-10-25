use melior::{
    Context,
    ir::{
        Module, RegionLike,
        block::BlockLike,
        operation::{OperationLike, OperationRef},
    },
    pass::PassManager,
    utility::parse_pass_pipeline,
};
use mlir_sys::{MlirContext, MlirModule};
use std::{
    cell::RefCell,
    collections::BTreeMap,
    ffi::{CString, c_char},
    rc::Rc,
    sync::Once,
};

unsafe extern "C" {
    fn qurts_register_dialects(context: MlirContext);
    fn qurts_register_passes();
    fn qurts_statevector(
        module: MlirModule,
        function: *const c_char,
        re: *mut f64,
        im: *mut f64,
        capacity: i64,
    ) -> i64;
}

pub const INFER_DROPS: &str = "builtin.module(func.func(qurts-infer-drops))";
pub const TO_QCO: &str = "builtin.module(convert-mlrd-to-qco)";
pub const CONTROLLED_CONJUGATION: &str = "builtin.module(qurts-controlled-conjugation)";
pub const MQT_OPTIMIZE: &str = "builtin.module(mqt-qco-default)";
pub const UNROLL_MODIFIERS: &str = "builtin.module(unroll-modifiers,canonicalize)";

pub fn register_dialects(context: &Context) {
    unsafe { qurts_register_dialects(context.to_raw()) }
}

pub fn register_passes() {
    static ONCE: Once = Once::new();
    ONCE.call_once(|| unsafe { qurts_register_passes() });
}

/// `lower::default_context()` plus the `qco`/`mqt` dialects and every pass this crate exposes.
pub fn context() -> Context {
    let context = lower::default_context();
    register_dialects(&context);
    register_passes();
    context
}

pub fn run_pipeline(context: &Context, module: &mut Module, pipeline: &str) -> Result<(), String> {
    let diagnostics = Rc::new(RefCell::new(Vec::new()));
    let sink = diagnostics.clone();
    let handler = context.attach_diagnostic_handler(move |diagnostic| {
        sink.borrow_mut().push(diagnostic.to_string());
        true
    });
    let manager = PassManager::new(context);
    let result = parse_pass_pipeline(manager.as_operation_pass_manager(), pipeline)
        .map_err(|error| error.to_string())
        .and_then(|()| manager.run(module).map_err(|_| format!("pipeline `{pipeline}` failed")));
    context.detach_diagnostic_handler(handler);
    result.map_err(|error| {
        let messages = diagnostics.take();
        if messages.is_empty() { error } else { messages.join("\n") }
    })
}

pub fn reparse<'c>(context: &'c Context, module: &Module) -> Module<'c> {
    Module::parse(context, &module.as_operation().to_string()).expect("printed module reparses")
}

/// Statevector of `function` after MQT's `unroll-modifiers`, run on a copy: the DD simulator
/// only takes dense matrices for nested modifiers whose wires line up with the body's, so
/// nested `qco.ctrl`/`qco.inv` straight out of `convert-mlrd-to-qco` must be flattened first.
pub fn simulate(context: &Context, module: &Module, function: &str) -> Result<Vec<(f64, f64)>, String> {
    let mut copy = reparse(context, module);
    run_pipeline(context, &mut copy, UNROLL_MODIFIERS)?;
    statevector(&copy, function).map_err(|code| format!("simulation failed with code {code}"))
}

pub fn statevector(module: &Module, function: &str) -> Result<Vec<(f64, f64)>, i64> {
    let name = CString::new(function).expect("function name has no NUL");
    let count = unsafe {
        qurts_statevector(module.to_raw(), name.as_ptr(), std::ptr::null_mut(), std::ptr::null_mut(), 0)
    };
    if count < 0 {
        return Err(count);
    }
    let mut re = vec![0.0; count as usize];
    let mut im = vec![0.0; count as usize];
    let written =
        unsafe { qurts_statevector(module.to_raw(), name.as_ptr(), re.as_mut_ptr(), im.as_mut_ptr(), count) };
    if written != count {
        return Err(written);
    }
    Ok(re.into_iter().zip(im).collect())
}

/// |<a|b>|^2, after discarding qubits that are exactly |0> in the larger state until the
/// dimensions match (MQT's cleanup drops allocated-but-unused qubits entirely).
pub fn fidelity(a: &[(f64, f64)], b: &[(f64, f64)]) -> f64 {
    let (mut a, mut b) = (a.to_vec(), b.to_vec());
    while a.len() != b.len() {
        let larger = if a.len() > b.len() { &mut a } else { &mut b };
        let Some(reduced) = drop_zero_qubit(larger) else { return 0.0 };
        *larger = reduced;
    }
    let (mut re, mut im) = (0.0, 0.0);
    for ((ar, ai), (br, bi)) in a.iter().zip(&b) {
        re += ar * br + ai * bi;
        im += ar * bi - ai * br;
    }
    re * re + im * im
}

fn drop_zero_qubit(state: &[(f64, f64)]) -> Option<Vec<(f64, f64)>> {
    let qubits = state.len().trailing_zeros();
    (0..qubits).find_map(|k| {
        let bit = 1usize << k;
        let settled = state.iter().enumerate().all(|(i, (re, im))| i & bit == 0 || (re.abs() < 1e-12 && im.abs() < 1e-12));
        settled.then(|| state.iter().enumerate().filter(|(i, _)| i & bit == 0).map(|(_, amp)| *amp).collect())
    })
}

pub fn operation_counts(module: &Module) -> BTreeMap<String, usize> {
    let mut counts = BTreeMap::new();
    count_ops(module.as_operation(), &mut counts);
    counts
}

fn count_ops(op: OperationRef, counts: &mut BTreeMap<String, usize>) {
    for i in 0..op.region_count() {
        let region = op.region(i).expect("region index in range");
        let mut block = region.first_block();
        while let Some(current) = block {
            let mut next = current.first_operation();
            while let Some(nested) = next {
                *counts.entry(nested.name().as_string_ref().as_str().unwrap().to_string()).or_default() += 1;
                count_ops(nested, counts);
                next = nested.next_in_block();
            }
            block = current.next_in_region();
        }
    }
}

pub fn nested_in(module: &Module, parent: &str, child: &str) -> bool {
    fn walk(op: OperationRef, parent: &str, child: &str, inside: bool) -> bool {
        for i in 0..op.region_count() {
            let region = op.region(i).expect("region index in range");
            let mut block = region.first_block();
            while let Some(current) = block {
                let mut next = current.first_operation();
                while let Some(nested) = next {
                    let name = nested.name().as_string_ref().as_str().unwrap().to_string();
                    if inside && name == child {
                        return true;
                    }
                    if walk(nested, parent, child, inside || name == parent) {
                        return true;
                    }
                    next = nested.next_in_block();
                }
                block = current.next_in_region();
            }
        }
        false
    }
    walk(module.as_operation(), parent, child, false)
}

pub fn gate_summary(module: &Module) -> String {
    let counts = operation_counts(module);
    let total: usize = counts
        .iter()
        .filter(|(name, _)| name.starts_with("qco.") && !matches!(name.as_str(), "qco.alloc" | "qco.sink" | "qco.yield" | "qco.measure"))
        .map(|(_, count)| count)
        .sum();
    let mut summary = format!("{total} gate ops");
    for (name, count) in counts.iter().filter(|(name, _)| name.starts_with("qco.")) {
        summary.push_str(&format!("\n  {name}: {count}"));
    }
    summary
}
