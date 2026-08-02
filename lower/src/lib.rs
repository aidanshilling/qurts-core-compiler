pub mod block;
pub mod cst_to_qauc;
pub mod cst_to_qduc;
pub mod env;
pub mod error;
pub mod expr;
pub mod function;
pub mod signature;
pub mod ty;
pub mod value;

use error::LowerError;
use function::{collect_signature, function_name, lower_function};
use signature::FunctionSignature;

use melior::{
    Context,
    dialect::DialectRegistry,
    ir::{Location, Module, block::BlockLike},
    utility::register_all_dialects,
};
use parser::Rule;
use pest::iterators::Pairs;
use std::collections::HashMap;

/// A `Context` with `func`/`arith`/`scf` (and `qduc`/`qauc`) registered — everything
/// `lower_program` needs. Shared by tests, examples, and any consumer (e.g. `playground`).
pub fn default_context() -> Context {
    let context = Context::new();
    let registry = DialectRegistry::new();
    register_all_dialects(&registry);
    context.append_dialect_registry(&registry);
    context.load_all_available_dialects();
    qduc::dialect::register(&context);
    qauc::dialect::register(&context);
    context
}

pub struct Lowerer<'c, 's> {
    pub context: &'c Context,
    pub signatures: &'s HashMap<String, FunctionSignature<'c>>,
}

pub struct LoweredProgram<'c> {
    pub module: Module<'c>,
    pub errors: Vec<(String, LowerError)>,
}

/// CST -> plain MLIR (`func`/`arith`/`scf`). Lifetimes/ownership are out of scope
/// (pass 2/3); functions needing them are skipped and recorded in `errors`.
pub fn lower_program<'c>(context: &'c Context, pairs: Pairs<Rule>) -> LoweredProgram<'c> {
    let function_pairs: Vec<_> = pairs
        .filter(|pair| pair.as_rule() == Rule::function)
        .collect();

    let mut signatures = HashMap::new();
    let mut errors = Vec::new();
    for function_pair in &function_pairs {
        let name = function_name(function_pair);
        match collect_signature(context, function_pair) {
            Ok((name, signature)) => {
                signatures.insert(name, signature);
            }
            Err(error) => errors.push((name, error)),
        }
    }

    let lowerer = Lowerer {
        context,
        signatures: &signatures,
    };
    let module = Module::new(Location::unknown(context));

    for function_pair in function_pairs {
        let name = function_name(&function_pair);
        let Some(signature) = signatures.get(&name) else {
            continue; // signature collection already failed and was recorded above
        };
        match lower_function(&lowerer, signature, function_pair) {
            Ok(op) => {
                module.body().append_operation(op);
            }
            Err(error) => errors.push((name, error)),
        }
    }

    LoweredProgram { module, errors }
}
