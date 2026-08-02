use crate::error::LowerError;
use melior::{
    Context,
    ir::{Block, Location, Type, Value, block::BlockLike, operation::OperationBuilder},
};

pub fn lifetime_type(context: &Context) -> Result<Type<'_>, LowerError> {
    Type::parse(context, "!qduc.lt").ok_or_else(|| LowerError::UnsupportedType("!qduc.lt".into()))
}

pub fn newlft<'c>(
    context: &'c Context,
    block: &Block<'c>,
    location: Location<'c>,
) -> Result<Value<'c, 'c>, LowerError> {
    let lifetime_type = lifetime_type(context)?;
    let op = block.append_operation(
        OperationBuilder::new("qduc.newlft", location)
            .add_results(&[lifetime_type])
            .build()?,
    );
    Ok(op.result(0)?.into())
}

pub fn end<'c>(
    block: &Block<'c>,
    lifetime: Value<'c, '_>,
    location: Location<'c>,
) -> Result<(), LowerError> {
    block.append_operation(
        OperationBuilder::new("qduc.end", location)
            .add_operands(&[lifetime])
            .build()?,
    );
    Ok(())
}
