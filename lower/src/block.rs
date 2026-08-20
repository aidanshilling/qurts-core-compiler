use super::{cst_to_qauc, cst_to_qduc, env::Env, error::LowerError, expr::lower_expr, value::LoweredValue};
use crate::Lowerer;
use melior::ir::{Block, Location};
use parser::Rule;
use pest::iterators::Pair;

/// Walks a CST `block`'s `stmt*` + optional trailing `expr`, appending operations
/// into `mlir_block`. Shared by function bodies, if/qif branches, and nested block-exprs.
pub fn lower_block_body<'c>(
    lowerer: &Lowerer<'c, '_>,
    env: &mut Env<'c>,
    mlir_block: &Block<'c>,
    block_pair: Pair<Rule>,
) -> Result<Option<LoweredValue<'c>>, LowerError> {
    let mut trailing = None;
    for child in block_pair.into_inner() {
        match child.as_rule() {
            Rule::stmt => lower_stmt(lowerer, env, mlir_block, child)?,
            Rule::expr => trailing = Some(lower_expr(lowerer, env, mlir_block, child)?),
            rule => unreachable!("unexpected block child {rule:?}"),
        }
    }
    Ok(trailing)
}

fn lower_stmt<'c>(
    lowerer: &Lowerer<'c, '_>,
    env: &mut Env<'c>,
    mlir_block: &Block<'c>,
    stmt_pair: Pair<Rule>,
) -> Result<(), LowerError> {
    let text = stmt_pair.as_str().to_string();
    let inner = stmt_pair.into_inner().next().expect("stmt always has exactly one child");

    match inner.as_rule() {
        Rule::let_stmt => {
            let mut children = inner.into_inner();
            let name = children.next().expect("let_stmt has an ident").as_str().to_string();
            let _ty = children.next().expect("let_stmt has a ty");
            let expr_pair = children.next().expect("let_stmt has an expr");
            let value = lower_expr(lowerer, env, mlir_block, expr_pair)?;
            env.define(name, value);
            Ok(())
        }
        Rule::expr_stmt => {
            let expr_pair = inner.into_inner().next().expect("expr_stmt has an expr");
            lower_expr(lowerer, env, mlir_block, expr_pair)?;
            Ok(())
        }
        Rule::newlft_stmt => {
            let location = Location::unknown(lowerer.context);
            let lifetime_pair = inner.into_inner().next().expect("newlft_stmt has a lifetime");
            let name = lifetime_var_name(lifetime_pair, &text)?;
            let token = cst_to_qduc::newlft(lowerer.context, mlir_block, location)?;
            if !env.open_lifetime(name.clone(), token) {
                return Err(LowerError::LifetimeAlreadyOpen(format!("'{name}")));
            }
            Ok(())
        }
        Rule::endlft_stmt => {
            let location = Location::unknown(lowerer.context);
            let lifetime_pair = inner.into_inner().next().expect("endlft_stmt has a lifetime");
            let name = lifetime_var_name(lifetime_pair, &text)?;
            let token = env
                .close_lifetime(&name)
                .ok_or_else(|| LowerError::UnknownLifetime(format!("'{name}")))?;
            cst_to_qduc::end(mlir_block, token.as_value(), location)?;
            Ok(())
        }
        Rule::borrow_stmt => {
            let location = Location::unknown(lowerer.context);
            let mut children = inner.into_inner();
            let new_name = children.next().expect("borrow_stmt has a new ident").as_str().to_string();
            let lifetime_pair = children.next().expect("borrow_stmt has a lifetime");
            let source_name = children.next().expect("borrow_stmt has a source ident").as_str().to_string();

            let lifetime_name = lifetime_var_name(lifetime_pair, &text)?;
            let lifetime = env
                .lifetime(&lifetime_name)
                .ok_or_else(|| LowerError::UnknownLifetime(format!("'{lifetime_name}")))?;

            let value = env
                .lookup(&source_name)
                .ok_or(LowerError::UndefinedVariable(source_name))?
                .as_single()
                .ok_or(LowerError::UnsupportedStmt(Rule::borrow_stmt, text))?;

            let result =
                cst_to_qauc::borrow(lowerer.context, mlir_block, value, lifetime.as_value(), location)?;
            env.define(new_name, LoweredValue::single(result));
            Ok(())
        }
        rule => unreachable!("unexpected stmt variant {rule:?}"),
    }
}

/// Rejects `'0`/`'static` as newlft/endlft/borrow_stmt lifetime targets.
fn lifetime_var_name(lifetime_pair: Pair<Rule>, stmt_text: &str) -> Result<String, LowerError> {
    let variant = lifetime_pair.into_inner().next().expect("lifetime has exactly one variant");
    match variant.as_rule() {
        Rule::lifetime_var => Ok(variant.as_str().to_string()),
        Rule::lifetime_empty | Rule::lifetime_static => {
            Err(LowerError::UnsupportedLifetime(variant.as_rule(), stmt_text.to_string()))
        }
        rule => unreachable!("unexpected lifetime variant {rule:?}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::default_context as test_context;
    use melior::ir::{Location, Module, operation::OperationLike};
    use pest::Parser;
    use std::collections::HashMap;

    fn lower_top_block<'c>(
        lowerer: &Lowerer<'c, '_>,
        source: &str,
    ) -> (Result<Option<LoweredValue<'c>>, LowerError>, Module<'c>) {
        let module = Module::new(Location::unknown(lowerer.context));
        let mut env = Env::new();
        let pair = parser::QurtsParser::parse(Rule::block, source).unwrap().next().unwrap();
        let result = lower_block_body(lowerer, &mut env, &module.body(), pair);
        (result, module)
    }

    #[test]
    fn lowers_newlft_endlft() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer { context: &context, signatures: &signatures };
        let (result, module) = lower_top_block(&lowerer, "{ newlft 'a; endlft 'a; }");
        result.unwrap();
        let text = module.as_operation().to_string();
        assert!(text.contains("qduc.newlft"), "{text}");
        assert!(text.contains("qduc.end"), "{text}");
        assert!(module.as_operation().verify());
    }

    #[test]
    fn lowers_crossing_lifetimes() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer { context: &context, signatures: &signatures };
        let (result, module) =
            lower_top_block(&lowerer, "{ newlft 'a; newlft 'b; endlft 'a; endlft 'b; }");
        result.unwrap();
        assert!(module.as_operation().verify());
    }

    #[test]
    fn rejects_double_open_lifetime() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer { context: &context, signatures: &signatures };
        let (result, _module) = lower_top_block(&lowerer, "{ newlft 'a; newlft 'a; }");
        assert!(matches!(result, Err(LowerError::LifetimeAlreadyOpen(name)) if name == "'a"));
    }

    #[test]
    fn rejects_endlft_unknown_lifetime() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer { context: &context, signatures: &signatures };
        let (result, _module) = lower_top_block(&lowerer, "{ endlft 'a; }");
        assert!(matches!(result, Err(LowerError::UnknownLifetime(name)) if name == "'a"));
    }

    #[test]
    fn rejects_newlft_static() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer { context: &context, signatures: &signatures };
        let (result, _module) = lower_top_block(&lowerer, "{ newlft 'static; }");
        assert!(matches!(
            result,
            Err(LowerError::UnsupportedLifetime(Rule::lifetime_static, _))
        ));
    }

    #[test]
    fn lowers_borrow_stmt() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer { context: &context, signatures: &signatures };
        let (result, module) = lower_top_block(
            &lowerer,
            "{ let x : bool = true; newlft 'a; let y = &'a x; endlft 'a; }",
        );
        result.unwrap();
        let text = module.as_operation().to_string();
        assert!(text.contains("qauc.borrow"), "{text}");
        assert!(module.as_operation().verify());
    }

    #[test]
    fn rejects_borrow_of_undefined_variable() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer { context: &context, signatures: &signatures };
        let (result, _module) =
            lower_top_block(&lowerer, "{ newlft 'a; let y = &'a x; endlft 'a; }");
        assert!(matches!(result, Err(LowerError::UndefinedVariable(name)) if name == "x"));
    }

    #[test]
    fn rejects_borrow_with_unknown_lifetime() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer { context: &context, signatures: &signatures };
        let (result, _module) =
            lower_top_block(&lowerer, "{ let x : bool = true; let y = &'a x; }");
        assert!(matches!(result, Err(LowerError::UnknownLifetime(name)) if name == "'a"));
    }

    #[test]
    fn rejects_borrow_of_tuple() {
        let context = test_context();
        let signatures = HashMap::new();
        let lowerer = Lowerer { context: &context, signatures: &signatures };
        let (result, _module) = lower_top_block(
            &lowerer,
            "{ let x : (bool, bool) = (true, false); newlft 'a; let y = &'a x; endlft 'a; }",
        );
        assert!(matches!(result, Err(LowerError::UnsupportedStmt(Rule::borrow_stmt, _))));
    }
}
