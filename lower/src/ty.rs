use crate::{cst_to_qauc, error::LowerError, value::Shape};
use melior::{
    Context,
    ir::{Type, r#type::IntegerType},
};
use parser::Rule;
use pest::iterators::Pair;

/// Structural shape of a CST `ty` node, independent of the fallible MLIR type
/// construction in `lower_ty` (used to un-flatten multi-value call results).
pub fn shape_of_ty(pair: Pair<Rule>) -> Shape {
    let inner = pair.into_inner().next().expect("ty always has exactly one child");
    match inner.as_rule() {
        Rule::unit_ty => Shape::Unit,
        Rule::product_ty => {
            let mut children = inner.into_inner();
            let lhs = shape_of_ty(children.next().expect("product_ty has two ty children"));
            let rhs = shape_of_ty(children.next().expect("product_ty has two ty children"));
            Shape::Tuple(vec![lhs, rhs])
        }
        _ => Shape::Single,
    }
}

/// Flattened MLIR types for a CST `ty` node: zero (unit), one (single), or N (tuple).
pub fn lower_ty<'c>(context: &'c Context, pair: Pair<Rule>) -> Result<Vec<Type<'c>>, LowerError> {
    let inner = pair.into_inner().next().expect("ty always has exactly one child");

    match inner.as_rule() {
        Rule::bool_ty => Ok(vec![IntegerType::new(context, 1).into()]),
        Rule::unit_ty => Ok(vec![]),
        Rule::qbit_ty => Ok(vec![cst_to_qauc::qbit_type(context)?]),
        Rule::product_ty => {
            let mut children = inner.into_inner();
            let lhs = lower_ty(context, children.next().expect("product_ty has two ty children"))?;
            let rhs = lower_ty(context, children.next().expect("product_ty has two ty children"))?;
            Ok(lhs.into_iter().chain(rhs).collect())
        }
        Rule::ref_ty | Rule::unique_ty => {
            let is_ref = inner.as_rule() == Rule::ref_ty;
            let mut children = inner.into_inner();
            let _lifetime = children.next().expect("ref_ty/unique_ty has a lifetime child");
            let value_ty = require_single(context, children.next().expect("ref_ty/unique_ty has a ty child"))?;
            let ty = if is_ref {
                cst_to_qauc::ref_type(context, value_ty)?
            } else {
                cst_to_qauc::unique_type(context, value_ty)?
            };
            Ok(vec![ty])
        }
        rule => unreachable!("unexpected ty variant {rule:?}"),
    }
}

fn require_single<'c>(context: &'c Context, pair: Pair<Rule>) -> Result<Type<'c>, LowerError> {
    let text = pair.as_str().to_string();
    match lower_ty(context, pair)?.as_slice() {
        [ty] => Ok(*ty),
        _ => Err(LowerError::UnsupportedType(format!(
            "reference/unique wrapping a non-single-valued type is not yet supported: {text}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_context as test_context;
    use pest::Parser;

    fn ty_pair(source: &'static str) -> Pair<'static, Rule> {
        parser::QurtsParser::parse(Rule::ty, source).unwrap().next().unwrap()
    }

    #[test]
    fn maps_bool_to_i1() {
        let context = test_context();
        let types = lower_ty(&context, ty_pair("bool")).unwrap();
        assert_eq!(types, vec![IntegerType::new(&context, 1).into()]);
    }

    #[test]
    fn maps_unit_to_zero_types() {
        let context = test_context();
        assert_eq!(lower_ty(&context, ty_pair("()")).unwrap(), vec![]);
    }

    #[test]
    fn maps_qbit_to_qauc_qbit() {
        let context = test_context();
        let types = lower_ty(&context, ty_pair("qbit")).unwrap();
        assert_eq!(types, vec![Type::parse(&context, "!qauc.qbit").unwrap()]);
    }

    #[test]
    fn maps_product_to_two_flattened_types() {
        let context = test_context();
        let types = lower_ty(&context, ty_pair("(bool, qbit)")).unwrap();
        assert_eq!(types.len(), 2);
        assert!(matches!(shape_of_ty(ty_pair("(bool, qbit)")), Shape::Tuple(shapes) if shapes.len() == 2));
    }

    #[test]
    fn maps_ref_and_unique_to_qauc_types() {
        let context = test_context();
        let ref_types = lower_ty(&context, ty_pair("&'a bool")).unwrap();
        assert_eq!(ref_types, vec![Type::parse(&context, "!qauc.ref<!qduc.lt, i1>").unwrap()]);

        let unique_types = lower_ty(&context, ty_pair("#'a qbit")).unwrap();
        assert_eq!(
            unique_types,
            vec![Type::parse(&context, "!qauc.unique<!qduc.lt, !qauc.qbit>").unwrap()]
        );
    }

    #[test]
    fn rejects_reference_to_tuple() {
        let context = test_context();
        assert!(lower_ty(&context, ty_pair("&'a (bool, qbit)")).is_err());
    }
}
