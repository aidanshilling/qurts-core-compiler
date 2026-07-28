use pest::iterators::{Pair, Pairs};
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "qurts.pest"]
pub struct QurtsParser;

pub fn format_parse_tree(pairs: Pairs<Rule>) -> String {
    let mut out = String::new();
    for pair in pairs {
        format_pair(pair, 0, &mut out);
    }
    out
}

fn format_pair(pair: Pair<Rule>, depth: usize, out: &mut String) {
    let rule = pair.as_rule();
    let text = pair.as_str();
    let indent = "  ".repeat(depth);
    let children: Vec<_> = pair.into_inner().collect();
    if children.is_empty() {
        out.push_str(&format!("{indent}{rule:?} {text:?}\n"));
    } else {
        out.push_str(&format!("{indent}{rule:?}\n"));
        for child in children {
            format_pair(child, depth + 1, out);
        }
    }
}

#[cfg(test)]
mod tests {
    use pest::Parser;
    use super::*;

    fn ok(src: &str) {
        assert!(QurtsParser::parse(Rule::program, src).is_ok(), "expected ok:\n  {src}");
    }

    fn err(src: &str) {
        assert!(QurtsParser::parse(Rule::program, src).is_err(), "expected err:\n  {src}");
    }

    #[test]
    fn fn_empty_body() {
        ok("fn test() -> bool {}");
    }

    #[test]
    fn fn_invalid_keyword() {
        err("func 1-2 () {}");
    }

    #[test]
    fn fn_with_params() {
        ok("fn f(x : bool, y : qbit) -> bool { x }");
    }

    #[test]
    fn fn_returning_unit() {
        ok("fn f() -> () { () }");
    }

    #[test]
    fn multiple_functions() {
        ok("fn f() -> bool { true } fn g() -> bool { false }");
    }

    #[test]
    fn let_stmt() {
        ok("fn f() -> bool { let x : bool = true; x }");
    }

    #[test]
    fn let_stmt_qbit_ty() {
        ok("fn f<'a>(x : #'a qbit) -> #'a qbit { let y : #'a qbit = x; y }");
    }

    #[test]
    fn borrow_stmt() {
        ok("fn f<'a>(x : bool) -> () { let y = &'a x; () }");
    }

    #[test]
    fn borrow_stmt_empty_lifetime() {
        ok("fn f(x : bool) -> () { let y = &'0 x; () }");
    }

    #[test]
    fn borrow_stmt_static_lifetime() {
        ok("fn f(x : bool) -> () { let y = &'static x; () }");
    }

    #[test]
    fn expr_stmt() {
        ok("fn f() -> () { g(); () }");
    }

    #[test]
    fn multiple_stmts() {
        ok("fn f() -> bool { let x : bool = true; let y : bool = false; x }");
    }

    #[test]
    fn expr_lit_true() {
        ok("fn f() -> bool { true }");
    }

    #[test]
    fn expr_lit_false() {
        ok("fn f() -> bool { false }");
    }

    #[test]
    fn expr_lit_unit() {
        ok("fn f() -> () { () }");
    }

    #[test]
    fn expr_ident() {
        ok("fn f(x : bool) -> bool { x }");
    }

    #[test]
    fn expr_tuple() {
        ok("fn f(x : bool, y : qbit) -> (bool, qbit) { (x, y) }");
    }

    #[test]
    fn expr_call_no_args() {
        ok("fn f() -> bool { g() }");
    }

    #[test]
    fn expr_call_with_args() {
        ok("fn f(x : bool, y : bool) -> bool { g(x, y) }");
    }

    #[test]
    fn expr_if() {
        ok("fn f(x : bool) -> bool { if x { true } else { false } }");
    }

    #[test]
    fn expr_if_no_else() {
        ok("fn f(x : bool) -> () { if x { () } }");
    }

    #[test]
    fn expr_if_call_condition() {
        ok("fn f() -> bool { if g() { true } else { false } }");
    }

    #[test]
    fn ty_product() {
        ok("fn f(x : bool, y : qbit) -> (bool, qbit) { (x, y) }");
    }

    #[test]
    fn ty_ref() {
        ok("fn f<'a>(x : &'a bool) -> &'a bool { x }");
    }

    #[test]
    fn ty_unique() {
        ok("fn f<'a>(x : #'a qbit) -> #'a qbit { x }");
    }

    #[test]
    fn ty_empty_lifetime() {
        ok("fn f(x : #'0 qbit) -> #'0 qbit { x }");
    }

    #[test]
    fn ty_static_lifetime() {
        ok("fn f(x : #'static bool) -> #'static bool { x }");
    }

    #[test]
    fn single_lifetime() {
        ok("fn test<'a>() -> bool {}");
    }

    #[test]
    fn multiple_lifetimes() {
        ok("fn f<'a, 'b>(x : &'a bool, y : &'b bool) -> () { () }");
    }

    #[test]
    fn lifetime_ordering_constraint() {
        ok("fn f<'a <= 'b>() -> bool { true }");
    }

    #[test]
    fn lifetime_inequality_constraint() {
        ok("fn test<'b != '0>() -> qbit {}");
    }

    #[test]
    fn lifetime_mixed_constraints() {
        ok("fn f<'a, 'b, 'a <= 'b>(x : #'a qbit) -> #'b qbit { x }");
    }

    #[test]
    fn reject_or_op() {
        err("fn f(x : bool, y : bool) -> bool { x || y }");
    }

    #[test]
    fn reject_and_op() {
        err("fn f(x : bool, y : bool) -> bool { x && y }");
    }

    #[test]
    fn reject_eq_op() {
        err("fn f(x : bool, y : bool) -> bool { x == y }");
    }

    #[test]
    fn reject_neq_op() {
        err("fn f(x : bool, y : bool) -> bool { x != y }");
    }

    #[test]
    fn reject_not_op() {
        err("fn f(x : bool) -> bool { !x }");
    }

    #[test]
    fn reject_deref_op() {
        err("fn f<'a>(x : &'a bool) -> bool { *x }");
    }

    #[test]
    fn reject_borrow_as_expr() {
        err("fn f<'a>(x : bool) -> &'a bool { &'a x }");
    }

    #[test]
    fn reject_unique_as_expr() {
        err("fn f<'a>(x : bool) -> #'a bool { #'a x }");
    }

    #[test]
    fn reject_let_no_type() {
        err("fn f() -> bool { let x = true; x }");
    }

    #[test]
    fn expr_measurement() {
        ok("fn f(x : qbit) -> bool { meas(x) }");
    }

    #[test]
    fn expr_unitary() {
        ok("fn f(x : qbit) -> qbit { H(x) }");
    }

    #[test]
    fn expr_lifted_zero_arity_ident() {
        ok("fn f() -> qbit { [new]() }");
    }

    #[test]
    fn expr_lifted_zero_arity_qstate_const() {
        ok("fn f() -> qbit { [0]() }");
        ok("fn f() -> qbit { [1]() }");
    }

    #[test]
    fn expr_lifted_one_arg() {
        ok("fn f(x : qbit) -> qbit { [X](x) }");
    }

    #[test]
    fn reject_lifted_multi_digit_qstate_const() {
        err("fn f() -> qbit { [2]() }");
    }

    #[test]
    fn call_expr_zero_and_multi_arg_still_work() {
        ok("fn f() -> bool { g() }");
        ok("fn f(x : bool, y : bool) -> bool { g(x, y) }");
    }

    #[test]
    fn newlft_endlft_stmts() {
        ok("fn f() -> bool { newlft 'a; endlft 'a; true }");
    }

    #[test]
    fn reject_endlft_without_semicolon() {
        err("fn f() -> bool { endlft 'a }");
    }

    #[test]
    fn reject_endlft_without_lifetime() {
        err("fn f() -> bool { endlft; true }");
    }

    #[test]
    fn reject_keywords_as_idents() {
        err("fn f() -> bool { let; true }");
        err("fn f() -> bool { if; true }");
        err("fn f() -> bool { newlft; true }");
        err("fn f(let : bool) -> bool { let }");
    }

    #[test]
    fn keyword_prefix_is_still_a_valid_ident() {
        ok("fn f(letter : bool) -> bool { letter }");
        ok("fn iffy() -> bool { iffy() }");
    }

    #[test]
    fn expr_qif() {
        ok("fn f<'a>(r : &'a bool) -> bool { qif r { true } else { false } }");
    }

    #[test]
    fn expr_qif_with_lifted() {
        ok("fn f<'a>(r : &'a qbit) -> qbit { qif r { let z : qbit = [1](); z } else { let z : qbit = [0](); z } }");
    }

    #[test]
    fn reject_qif_without_else() {
        err("fn f<'a>(r : &'a bool) -> bool { qif r { true } }");
    }

    #[test]
    fn reject_qif_non_ident_condition() {
        err("fn f() -> bool { qif (true) { true } else { false } }");
    }

    #[test]
    fn reject_qif_as_ident() {
        err("fn f(qif : bool) -> bool { qif }");
    }
}
