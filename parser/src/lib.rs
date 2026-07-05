use pest::Parser;
use pest_derive::Parser;

#[derive(Parser)]
#[grammar = "qurts.pest"]
pub struct QurtsParser;

#[cfg(test)]
mod tests {
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
}
