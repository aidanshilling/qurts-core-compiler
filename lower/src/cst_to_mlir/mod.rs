pub mod block;
pub mod env;
pub mod error;
pub mod expr;
pub mod function;
pub mod signature;
#[cfg(test)]
pub(crate) mod test_util;
pub mod ty;
pub mod value;

use error::LowerError;
use function::{collect_signature, lower_function};
use melior::{
    Context,
    ir::{Location, Module, block::BlockLike},
};
use parser::Rule;
use pest::iterators::Pairs;
use signature::FunctionSignature;
use std::collections::HashMap;

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
    let function_pairs: Vec<_> = pairs.filter(|pair| pair.as_rule() == Rule::function).collect();

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

    let lowerer = Lowerer { context, signatures: &signatures };
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

fn function_name(function_pair: &pest::iterators::Pair<Rule>) -> String {
    function_pair.clone().into_inner().next().expect("function has a name").as_str().to_string()
}
