use super::{
    block::lower_block_body,
    env::Env,
    error::LowerError,
    signature::FunctionSignature,
    ty::{lower_ty, shape_of_ty},
    value::{LoweredValue, Shape},
};
use crate::Lowerer;
use melior::{
    Context,
    dialect::func,
    ir::{
        Block, Location, Operation, Region, RegionLike, Value,
        attribute::{StringAttribute, TypeAttribute},
        block::BlockLike,
        r#type::FunctionType,
    },
};
use parser::Rule;
use pest::iterators::Pair;

fn param_list_pair(function_pair: Pair<Rule>) -> Pair<Rule> {
    let mut children = function_pair.into_inner();
    let signature_pair = children.nth(1).expect("function has a signature");
    let mut sig_children = signature_pair.into_inner();
    let first = sig_children.next().expect("signature has at least a param_list");
    if first.as_rule() == Rule::lifetime_preorder {
        sig_children.next().expect("signature has a param_list after preorder")
    } else {
        first
    }
}

/// First pass: collect a function's signature without lowering its body, so
/// forward-referenced calls resolve regardless of declaration order.
pub fn collect_signature<'c>(
    context: &'c Context,
    function_pair: &Pair<Rule>,
) -> Result<(String, FunctionSignature<'c>), LowerError> {
    let name = function_pair
        .clone()
        .into_inner()
        .next()
        .expect("function has a name")
        .as_str()
        .to_string();

    let mut param_names = Vec::new();
    let mut param_shapes = Vec::new();
    let mut param_types = Vec::new();
    for param in param_list_pair(function_pair.clone()).into_inner() {
        let mut param_children = param.into_inner();
        let ident = param_children.next().expect("param has an ident").as_str().to_string();
        let ty_pair = param_children.next().expect("param has a ty");
        param_shapes.push(shape_of_ty(ty_pair.clone()));
        param_types.extend(lower_ty(context, ty_pair)?);
        param_names.push(ident);
    }

    let signature_pair = function_pair.clone().into_inner().nth(1).expect("function has a signature");
    let return_ty_pair = signature_pair
        .into_inner()
        .last()
        .expect("signature always ends with a return ty");
    let result_shape = shape_of_ty(return_ty_pair.clone());
    let result_types = lower_ty(context, return_ty_pair)?;

    Ok((name, FunctionSignature { param_names, param_shapes, param_types, result_types, result_shape }))
}

/// Second pass: lower a function's body into a real `func.func`, given the
/// already-collected signature (so calls to any function in the program resolve).
pub fn lower_function<'c>(
    lowerer: &Lowerer<'c, '_>,
    signature: &FunctionSignature<'c>,
    function_pair: Pair<Rule>,
) -> Result<Operation<'c>, LowerError> {
    let mut children = function_pair.into_inner();
    let name = children.next().expect("function has a name").as_str().to_string();
    let _signature_pair = children.next().expect("function has a signature");
    let block_pair = children.next().expect("function has a block");

    let location = Location::unknown(lowerer.context);
    let arg_locations: Vec<_> = signature.param_types.iter().map(|ty| (*ty, location)).collect();
    let entry_block = Block::new(&arg_locations);

    let mut env = Env::new();
    let mut arg_index = 0;
    for (param_name, param_shape) in signature.param_names.iter().zip(&signature.param_shapes) {
        let values: Vec<Value> = (0..param_shape_len(param_shape))
            .map(|_| {
                let value = Value::from(
                    entry_block.argument(arg_index).expect("entry block has declared arg count"),
                );
                arg_index += 1;
                value
            })
            .collect();
        let mut values = values.into_iter();
        env.define(param_name.clone(), param_shape.unflatten(&mut values));
    }

    let trailing = lower_block_body(lowerer, &mut env, &entry_block, block_pair)?
        .unwrap_or(LoweredValue::Tuple(vec![]));
    let return_values = trailing.flatten();
    entry_block.append_operation(func::r#return(&return_values, location));

    let region = Region::new();
    region.append_block(entry_block);

    let function_type =
        FunctionType::new(lowerer.context, &signature.param_types, &signature.result_types);

    Ok(func::func(
        lowerer.context,
        StringAttribute::new(lowerer.context, &name),
        TypeAttribute::new(function_type.into()),
        region,
        &[],
        location,
    ))
}

fn param_shape_len(shape: &Shape) -> usize {
    match shape {
        Shape::Unit => 0,
        Shape::Single => 1,
        Shape::Tuple(shapes) => shapes.iter().map(param_shape_len).sum(),
    }
}
