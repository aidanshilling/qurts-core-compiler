use crate::error::LowerError;
use melior::{
    Context,
    ir::{Block, Location, Type, Value, ValueLike, block::BlockLike, operation::OperationBuilder},
};

pub fn qbit_type(context: &Context) -> Result<Type<'_>, LowerError> {
    parse_type(context, "!qauc.qbit")
}

pub fn ref_type<'c>(context: &'c Context, value_type: Type<'c>) -> Result<Type<'c>, LowerError> {
    parse_type(context, &format!("!qauc.ref<!qduc.lt, {value_type}>"))
}

pub fn unique_type<'c>(context: &'c Context, value_type: Type<'c>) -> Result<Type<'c>, LowerError> {
    parse_type(context, &format!("!qauc.unique<!qduc.lt, {value_type}>"))
}

fn parse_type<'c>(context: &'c Context, source: &str) -> Result<Type<'c>, LowerError> {
    Type::parse(context, source).ok_or_else(|| LowerError::UnsupportedType(source.to_string()))
}

pub fn borrow<'c>(
    context: &'c Context,
    block: &Block<'c>,
    value: Value<'c, '_>,
    lifetime: Value<'c, '_>,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, LowerError> {
    let ref_type = ref_type(context, value.r#type())?;
    let op = block.append_operation(
        OperationBuilder::new("qauc.borrow", location)
            .add_operands(&[value, lifetime])
            .add_results(&[ref_type])
            .build()?,
    );
    Ok(op.result(0)?.into())
}
