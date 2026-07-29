pub mod block;
pub mod env;
pub mod error;
pub mod expr;
pub mod function;
pub mod signature;
pub mod ty;
pub mod value;

use parser::Rule;

pub fn function_name(function_pair: &pest::iterators::Pair<Rule>) -> String {
    function_pair
        .clone()
        .into_inner()
        .next()
        .expect("function has a name")
        .as_str()
        .to_string()
}
