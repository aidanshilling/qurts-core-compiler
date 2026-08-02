use crate::error::LowerError;
use melior::{Context, ir::Type};

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
