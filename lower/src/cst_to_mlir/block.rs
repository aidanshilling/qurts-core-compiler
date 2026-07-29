use super::{env::Env, error::LowerError, expr::lower_expr, value::LoweredValue};
use crate::Lowerer;
use melior::ir::Block;
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
        Rule::borrow_stmt | Rule::newlft_stmt | Rule::endlft_stmt => {
            Err(LowerError::UnsupportedStmt(inner.as_rule(), text))
        }
        rule => unreachable!("unexpected stmt variant {rule:?}"),
    }
}
