mod common;

use common::expr;
use foxvm::ast::{ExprKind, UnOp};
use foxvm::parser::{parse_expression, parse_program};

fn e(src: &str) -> String {
    match parse_expression(src) {
        Ok(x) => expr(&x),
        Err(d) => panic!("failed to parse {src:?}: {d:#?}"),
    }
}

#[test]
fn arithmetic_precedence() {
    assert_eq!(e("1 + 2 * 3"), "(+ 1 (* 2 3))");
    assert_eq!(e("(1 + 2) * 3"), "(* (+ 1 2) 3)");
    assert_eq!(e("10 - 4 - 3"), "(- (- 10 4) 3)");
    assert_eq!(e("a / b % c"), "(% (/ A B) C)");
}

/// A sign binds tighter than the power it is raised to, and a chain of powers is worked left to
/// right - both the other way round from C, and both measured against Visual FoxPro 9, which
/// answers 4 to `-2 ^ 2` and 729 to `3 ^ 2 ^ 3`.
#[test]
fn unary_minus_binds_tighter_than_power_and_power_is_left_associative() {
    assert_eq!(e("-2 ^ 2"), "(^ (- 2) 2)");
    assert_eq!(e("2 ^ 3 ^ 2"), "(^ (^ 2 3) 2)");
    assert_eq!(e("2 ** 3"), "(^ 2 3)");
    assert_eq!(e("2 ^ -1"), "(^ 2 (- 1))");
    assert_eq!(e("-x * 2"), "(* (- X) 2)");
}

#[test]
fn not_binds_looser_than_comparison() {
    assert_eq!(e("NOT a = b"), "(! (= A B))");
    assert_eq!(e(".NOT. a = b"), "(! (= A B))");
    assert_eq!(e("!a = b"), "(! (= A B))");
    assert_eq!(e("!x"), "(! X)");
    assert_eq!(e("a = NOT b"), "(= A (! B))");
    assert_eq!(e("NOT a AND b"), "(AND (! A) B)");
}

#[test]
fn and_binds_tighter_than_or() {
    assert_eq!(e("a AND b OR c"), "(OR (AND A B) C)");
    assert_eq!(e("a OR b AND c"), "(OR A (AND B C))");
    assert_eq!(e("a .AND. b .OR. c"), "(OR (AND A B) C)");
    assert_eq!(e("a or b and c"), "(OR A (AND B C))");
}

#[test]
fn comparison_operators() {
    assert_eq!(e("\"a\" $ s"), "($ \"a\" S)");
    assert_eq!(e("a == b"), "(== A B)");
    assert_eq!(e("x <> y"), "(<> X Y)");
    assert_eq!(e("x # y"), "(<> X Y)");
    assert_eq!(e("x != y"), "(<> X Y)");
    assert_eq!(e("x < y"), "(< X Y)");
    assert_eq!(e("x <= y"), "(<= X Y)");
    assert_eq!(e("x > y"), "(> X Y)");
    assert_eq!(e("x >= y"), "(>= X Y)");
    // people write these the other way round and VFP reads them the same, which one of the
    // shipped samples relies on: `WHERE company <= lcHigh and company => lcLow`
    assert_eq!(e("x => y"), "(>= X Y)");
    assert_eq!(e("x =< y"), "(<= X Y)");
    assert_eq!(e("a + 1 = b * 2"), "(= (+ A 1) (* B 2))");
}

#[test]
fn literals() {
    assert_eq!(e("1.5"), "1.5");
    assert_eq!(e(".5"), "0.5");
    assert_eq!(e("0x10"), "16");
    assert_eq!(e("'it\"s'"), "\"it\\\"s\"");
    assert_eq!(e("[brackets]"), "\"brackets\"");
    assert_eq!(e(".T."), ".T.");
    assert_eq!(e(".f."), ".F.");
    assert_eq!(e(".NULL."), ".NULL.");
    assert_eq!(e("{^2024-01-31}"), "{^2024-01-31}");
    assert_eq!(e("{^2024-01-31 10:30}"), "{^2024-01-31 10:30:00}");
    assert_eq!(e("{}"), "{}");
    assert_eq!(e("{/:}"), "{/:}");
}

#[test]
fn member_chains_and_special_names() {
    assert_eq!(e("THISFORM.pgfMain.Page1.lblGreeting.Caption"), "THISFORM.PGFMAIN.PAGE1.LBLGREETING.CAPTION");
    assert_eq!(e("This.Parent.Name"), "THIS.PARENT.NAME");
    assert_eq!(e("THISFORMSET.Forms(1)"), "THISFORMSET.FORMS(1)");
    assert_eq!(e("_SCREEN.Caption"), "_SCREEN.CAPTION");
    assert_eq!(e("obj.Method(1, 2).Prop"), "OBJ.METHOD(1, 2).PROP");
    assert_eq!(e("x.t.y"), "X.T.Y");
}

#[test]
fn index_and_calls() {
    assert_eq!(e("a[1, 2]"), "A[1, 2]");
    assert_eq!(e("a[b[1]]"), "A[B[1]]");
    assert_eq!(e("f(1,,3)"), "F(1, _, 3)");
    assert_eq!(e("f(1,)"), "F(1, _)");
    assert_eq!(e("f()"), "F()");
    assert_eq!(e("f(@x)"), "F(@X)");
    assert_eq!(e("f(@x.y, 2)"), "F(@X.Y, 2)");
    assert_eq!(e("IIF(a, b, c)"), "IIF(A, B, C)");
    assert_eq!(e("DODEFAULT(1)"), "DODEFAULT(1)");
    assert_eq!(e("EVALUATE(\"x\")"), "EVALUATE(\"x\")");
    assert_eq!(e("arr(1, 2)"), "ARR(1, 2)");
    assert_eq!(e("obj.list[3].name"), "OBJ.LIST[3].NAME");
}

#[test]
fn brackets_are_a_subscript_or_a_string_depending_on_what_precedes_them() {
    // separated by spaces, and still a subscript: it follows something that can be indexed
    assert_eq!(e("a [ 1 ]"), "A[1]");
    assert_eq!(e("INT(VAL( aFontInfo [ 4 ] ))"), "INT(VAL(AFONTINFO[4]))");
    assert_eq!(e("This.aRGB [ 2 ]"), "THIS.ARGB[2]");
    assert_eq!(e("a [1, 2]"), "A[1, 2]");
    assert_eq!(e("f(1) [2]"), "F(1)[2]");
    // in operand position the brackets are VFP's third string delimiter
    assert_eq!(e("[hello]"), "\"hello\"");
    assert_eq!(e("x + [hello]"), "(+ X \"hello\")");
    assert_eq!(e("f([hello])"), "F(\"hello\")");
    // a literal cannot be indexed, so brackets after one stay a string
    assert_eq!(e("[a] + [b]"), "(+ \"a\" \"b\")");
    // a subscript that is not an expression is an error, not a silent string
    assert!(parse_expression("a [ 1 +  ]").is_err());
}

#[test]
fn macro_in_expression() {
    assert_eq!(e("&cmd"), "&CMD");
    assert_eq!(e("&cmd. + 1"), "(+ &CMD 1)");
    assert_eq!(e("\"x\" + &var"), "(+ \"x\" &VAR)");
}

#[test]
fn with_member_reads_as_a_member_of_whatever_with_is_open_on() {
    let out = parse_program("WITH THISFORM\n  x = .Caption + .Pages(1).Caption\n  .Refresh()\nENDWITH");
    assert!(out.diagnostics.is_empty(), "{:#?}", out.diagnostics);
    assert_eq!(
        common::dump(&out.program),
        "(with THISFORM (block (= X (+ <with>.CAPTION <with>.PAGES(1).CAPTION)) (expr <with>.REFRESH())))"
    );
    // whether a WITH is open is a question for the moment the line runs, not for the compiler:
    // Visual FoxPro compiles `CursorGetProp("sourcetype", .ALIAS)` in a method with no WITH in
    // it at all, and the Foundation Classes ship such a method
    assert_eq!(e(".Caption"), "<with>.CAPTION");
}

#[test]
fn at_only_in_arguments() {
    let err = parse_expression("@x + 1").unwrap_err();
    assert_eq!(err[0].message, "'@' is only valid in an argument list");
}

#[test]
fn whole_input_must_be_one_expression() {
    let err = parse_expression("1 + 2 3").unwrap_err();
    assert_eq!(err[0].message, "Syntax error: unexpected '3' after expression");
    let err = parse_expression("1 +").unwrap_err();
    assert_eq!(err[0].message, "Syntax error: expected expression, found end of line");
    let err = parse_expression("").unwrap_err();
    assert_eq!(err[0].message, "Syntax error: expected expression, found end of file");
    let err = parse_expression("f(1").unwrap_err();
    assert_eq!(err[0].message, "Syntax error: expected ',' or ')', found end of line");
}

#[test]
fn trailing_newline_and_comment_are_allowed() {
    assert_eq!(e("1 + 2 && sum\n"), "(+ 1 2)");
}

#[test]
fn spans_cover_expressions() {
    let x = parse_expression("  1 + foo(2)").unwrap();
    assert_eq!((x.span.start, x.span.end), (2, 12));
    let ExprKind::Binary { right, .. } = &x.kind else { panic!() };
    assert_eq!((right.span.start, right.span.end), (6, 12));
    let n = parse_expression("-x").unwrap();
    assert!(matches!(n.kind, ExprKind::Unary { op: UnOp::Neg, .. }));
    assert_eq!((n.span.start, n.span.end), (0, 2));
}
