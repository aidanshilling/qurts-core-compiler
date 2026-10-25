use super::cst_to_qauc;
use crate::error::LowerError;
use melior::{
    Context,
    ir::{
        Block, Identifier, Location, Region, Type, Value,
        attribute::StringAttribute,
        block::BlockLike,
        operation::OperationBuilder,
        r#type::IntegerType,
    },
};

pub fn lifted<'c>(
    context: &'c Context,
    block: &Block<'c>,
    gate: &str,
    operands: &[Value<'c, '_>],
    location: Location<'c>,
) -> Result<Vec<Value<'c, 'c>>, LowerError> {
    let gate_attr = StringAttribute::new(context, gate);
    let qbit = cst_to_qauc::qbit_type(context)?;
    let result_types = vec![qbit; operands.len().max(1)];
    let op = block.append_operation(
        OperationBuilder::new("mlrd.lifted", location)
            .add_attributes(&[(Identifier::new(context, "gate"), gate_attr.into())])
            .add_operands(operands)
            .add_results(&result_types)
            .build()?,
    );
    Ok((0..result_types.len())
        .map(|i| Value::from(op.result(i).expect("lifted produces declared result count")))
        .collect())
}

pub fn unitary<'c>(
    context: &'c Context,
    block: &Block<'c>,
    gate: &str,
    operands: &[Value<'c, '_>],
    location: Location<'c>,
) -> Result<Vec<Value<'c, 'c>>, LowerError> {
    let gate_attr = StringAttribute::new(context, gate);
    let qbit = cst_to_qauc::qbit_type(context)?;
    let result_types = vec![qbit; operands.len()];
    let op = block.append_operation(
        OperationBuilder::new("mlrd.unitary", location)
            .add_attributes(&[(Identifier::new(context, "gate"), gate_attr.into())])
            .add_operands(operands)
            .add_results(&result_types)
            .build()?,
    );
    Ok((0..operands.len())
        .map(|i| Value::from(op.result(i).expect("unitary produces one result per operand")))
        .collect())
}

pub fn meas<'c>(
    context: &'c Context,
    block: &Block<'c>,
    qbit: Value<'c, '_>,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, LowerError> {
    let op = block.append_operation(
        OperationBuilder::new("mlrd.meas", location)
            .add_operands(&[qbit])
            .add_results(&[IntegerType::new(context, 1).into()])
            .build()?,
    );
    Ok(op.result(0)?.into())
}

pub fn qif<'c>(
    block: &Block<'c>,
    condition: Value<'c, '_>,
    then_region: Region<'c>,
    else_region: Region<'c>,
    result_types: &[Type<'c>],
    location: Location<'c>,
) -> Result<Vec<Value<'c, 'c>>, LowerError> {
    let op = block.append_operation(
        OperationBuilder::new("mlrd.qif", location)
            .add_operands(&[condition])
            .add_regions([then_region, else_region])
            .add_results(result_types)
            .build()?,
    );
    Ok((0..result_types.len())
        .map(|i| Value::from(op.result(i).expect("qif produces declared result count")))
        .collect())
}

pub fn r#yield<'c>(
    block: &Block<'c>,
    values: &[Value<'c, '_>],
    location: Location<'c>,
) -> Result<(), LowerError> {
    block.append_operation(
        OperationBuilder::new("mlrd.yield", location)
            .add_operands(values)
            .build()?,
    );
    Ok(())
}
