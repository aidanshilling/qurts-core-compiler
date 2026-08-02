use super::{block::lower_block_body, env::Env, error::LowerError, value::LoweredValue};
use crate::Lowerer;
use melior::{
    dialect::{arith, func, scf},
    ir::{
        Block, Location, Region, RegionLike, Value, ValueLike,
        attribute::{FlatSymbolRefAttribute, IntegerAttribute},
        block::BlockLike,
        r#type::IntegerType,
    },
};
use parser::Rule;
use pest::iterators::Pair;

pub fn lower_expr<'c>(
    lowerer: &Lowerer<'c, '_>,
    env: &mut Env<'c>,
    block: &Block<'c>,
    pair: Pair<Rule>,
) -> Result<LoweredValue<'c>, LowerError> {
    let text = pair.as_str().to_string();
    let inner = pair
        .into_inner()
        .next()
        .expect("expr always has exactly one child");
    let location = Location::unknown(lowerer.context);

    match inner.as_rule() {
        Rule::lit_bool => {
            let value = inner.as_str() == "true";
            let attr =
                IntegerAttribute::new(IntegerType::new(lowerer.context, 1).into(), value as i64);
            let op =
                block.append_operation(arith::constant(lowerer.context, attr.into(), location));
            Ok(LoweredValue::single(op.result(0)?))
        }
        Rule::lit_unit => Ok(LoweredValue::Tuple(vec![])),
        Rule::tuple_expr => {
            let mut children = inner.into_inner();
            let lhs = lower_expr(
                lowerer,
                env,
                block,
                children.next().expect("tuple_expr has two exprs"),
            )?;
            let rhs = lower_expr(
                lowerer,
                env,
                block,
                children.next().expect("tuple_expr has two exprs"),
            )?;
            Ok(LoweredValue::Tuple(vec![lhs, rhs]))
        }
        Rule::ident => env
            .lookup(inner.as_str())
            .ok_or_else(|| LowerError::UndefinedVariable(inner.as_str().to_string())),
        Rule::call_expr => lower_call(lowerer, env, block, inner, location),
        Rule::if_expr => lower_if(lowerer, env, block, inner, location),
        Rule::block => {
            env.push_scope();
            let result = lower_block_body(lowerer, env, block, inner);
            env.pop_scope();
            Ok(result?.unwrap_or(LoweredValue::Tuple(vec![])))
        }
        Rule::meas_expr | Rule::unitary_expr | Rule::lifted_expr | Rule::qif_expr => {
            Err(LowerError::UnsupportedExpr(inner.as_rule(), text))
        }
        rule => unreachable!("unexpected expr variant {rule:?}"),
    }
}

fn lower_call<'c>(
    lowerer: &Lowerer<'c, '_>,
    env: &mut Env<'c>,
    block: &Block<'c>,
    pair: Pair<Rule>,
    location: Location<'c>,
) -> Result<LoweredValue<'c>, LowerError> {
    let mut children = pair.into_inner();
    let callee_name = children
        .next()
        .expect("call_expr has an ident")
        .as_str()
        .to_string();
    let arg_list = children.next().expect("call_expr has an arg_list");

    let mut operands: Vec<Value<'c, 'c>> = Vec::new();
    for arg in arg_list.into_inner() {
        operands.extend(lower_expr(lowerer, env, block, arg)?.flatten());
    }

    let signature = lowerer
        .signatures
        .get(&callee_name)
        .ok_or_else(|| LowerError::UndefinedVariable(callee_name.clone()))?;

    let callee = FlatSymbolRefAttribute::new(lowerer.context, &callee_name);
    let op = block.append_operation(func::call(
        lowerer.context,
        callee,
        &operands,
        &signature.result_types,
        location,
    ));

    let results: Vec<Value<'c, 'c>> = (0..signature.result_types.len())
        .map(|i| Value::from(op.result(i).expect("call produces declared result count")))
        .collect();
    let mut results = results.into_iter();
    Ok(signature.result_shape.unflatten(&mut results))
}

fn lower_if<'c>(
    lowerer: &Lowerer<'c, '_>,
    env: &mut Env<'c>,
    block: &Block<'c>,
    pair: Pair<Rule>,
    location: Location<'c>,
) -> Result<LoweredValue<'c>, LowerError> {
    let mut children = pair.into_inner();
    let condition_pair = children.next().expect("if_expr has a condition expr");
    let then_pair = children.next().expect("if_expr has a then block");
    let else_pair = children.next();

    let condition = lower_expr(lowerer, env, block, condition_pair)?
        .as_single()
        .expect("if condition lowers to a single bool value");

    let then_block = Block::new(&[]);
    env.push_scope();
    let then_result = lower_block_body(lowerer, env, &then_block, then_pair)?
        .unwrap_or(LoweredValue::Tuple(vec![]));
    env.pop_scope();
    let then_values = then_result.flatten();
    then_block.append_operation(scf::r#yield(&then_values, location));
    let result_types: Vec<_> = then_values.iter().map(ValueLike::r#type).collect();
    let then_region = Region::new();
    then_region.append_block(then_block);

    let else_block = Block::new(&[]);
    let else_result = if let Some(else_pair) = else_pair {
        env.push_scope();
        let result = lower_block_body(lowerer, env, &else_block, else_pair)?
            .unwrap_or(LoweredValue::Tuple(vec![]));
        env.pop_scope();
        result
    } else {
        LoweredValue::Tuple(vec![])
    };
    let else_values = else_result.flatten();
    else_block.append_operation(scf::r#yield(&else_values, location));
    let else_region = Region::new();
    else_region.append_block(else_block);

    let op = block.append_operation(scf::r#if(
        condition,
        &result_types,
        then_region,
        else_region,
        location,
    ));

    let results: Vec<Value<'c, 'c>> = (0..result_types.len())
        .map(|i| Value::from(op.result(i).expect("scf.if produces declared result count")))
        .collect();
    match results.as_slice() {
        [] => Ok(LoweredValue::Tuple(vec![])),
        [value] => Ok(LoweredValue::single(*value)),
        _ => Ok(LoweredValue::Tuple(
            results.into_iter().map(LoweredValue::single).collect(),
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_context as test_context;
    use melior::ir::{Module, operation::OperationLike};
    use pest::Parser;
    use std::collections::HashMap;

    fn lower_top_expr<'c>(
        lowerer: &Lowerer<'c, '_>,
        source: &str,
    ) -> (LoweredValue<'c>, Module<'c>) {
        let module = Module::new(Location::unknown(lowerer.context));
        let mut env = Env::new();
        let pair = parser::QurtsParser::parse(Rule::expr, source)
            .unwrap()
            .next()
            .unwrap();
        let value = lower_expr(lowerer, &mut env, &module.body(), pair).unwrap();
        (value, module)
    }

    #[test]
    fn lowers_bool_literal() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer {
            context: &context,
            signatures: &signatures,
        };
        let (value, _module) = lower_top_expr(&lowerer, "true");
        assert!(value.as_single().is_some());
    }

    #[test]
    fn lowers_tuple() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer {
            context: &context,
            signatures: &signatures,
        };
        let (value, _module) = lower_top_expr(&lowerer, "(true, false)");
        match value {
            LoweredValue::Tuple(values) => assert_eq!(values.len(), 2),
            LoweredValue::Single(_) => panic!("expected a tuple"),
        }
    }

    #[test]
    fn lowers_if_expr() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer {
            context: &context,
            signatures: &signatures,
        };
        let (value, module) = lower_top_expr(&lowerer, "if true { true } else { false }");
        assert!(value.as_single().is_some());
        assert!(module.as_operation().verify());
    }

    #[test]
    fn rejects_unitary_expr() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer {
            context: &context,
            signatures: &signatures,
        };
        let module = Module::new(Location::unknown(&context));
        let mut env = Env::new();
        let pair = parser::QurtsParser::parse(Rule::expr, "H(x)")
            .unwrap()
            .next()
            .unwrap();
        let result = lower_expr(&lowerer, &mut env, &module.body(), pair);
        assert!(matches!(
            result,
            Err(LowerError::UnsupportedExpr(Rule::unitary_expr, _))
        ));
    }
}
