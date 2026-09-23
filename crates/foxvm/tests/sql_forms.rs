//! Five spellings real Visual FoxPro applications use that the reference examples do not:
//! `USE IN SELECT('alias')`, `?lcName` parameters in local SQL, `INSERT INTO ... SELECT`,
//! `UNION [ALL]`, and names in quotes after `DO` and `CREATE TABLE`. Every one was found in a
//! CodeMine application of 60,000 lines and none of them is rare there.

mod common;
mod dbf_fixture;

use dbf_fixture::table;
use foxvm::mock_host::{MockHost, run_program};

fn host() -> MockHost {
    let parts = table(
        &[("CODE", b'C', 3, 0), ("QTY", b'N', 3, 0)],
        &[&["AAA", "1"], &["BBB", "2"], &["CCC", "3"]],
    );
    let mut host = MockHost::new();
    host.tables.insert("PARTS.DBF".into(), parts);
    host
}

fn run(src: &str) -> Vec<String> {
    let mut host = host();
    match run_program(src, &mut host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

// ----- parsing -------------------------------------------------------------------------------

#[test]
fn use_in_select_of_alias_is_a_call() {
    assert_eq!(common::ok("USE IN SELECT('orders')"), "(use - (in SELECT(\"orders\")))");
    assert_eq!(common::ok("USE IN (SELECT('orders'))"), "(use - (in SELECT(\"orders\")))");
    // a bare name is still an alias, as it always was
    assert_eq!(common::ok("USE IN orders"), "(use - (in \"orders\"))");
}

#[test]
fn question_mark_is_a_parameter_inside_a_select() {
    // measured: the mark is dropped and what follows is an ordinary expression
    assert_eq!(
        common::ok("SELECT a FROM t WHERE b = ?lcName INTO CURSOR q"),
        "(select (A) (from t:T) (where (= B LCNAME)) (into-cursor Q))"
    );
    assert_eq!(
        common::ok("SELECT a FROM t WHERE b = ?m.lcName INTO CURSOR q"),
        "(select (A) (from t:T) (where (= B M.LCNAME)) (into-cursor Q))"
    );
    assert_eq!(
        common::ok("SELECT a FROM t WHERE b = ?tblorder.ipkey INTO CURSOR q"),
        "(select (A) (from t:T) (where (= B TBLORDER.IPKEY)) (into-cursor Q))"
    );
    // measured: INSERT, UPDATE and DELETE - SQL refuse it, and so does an expression anywhere
    // else, where `?` is the print command and nothing more
    for src in ["INSERT INTO t (a) VALUES (?lcName)", "UPDATE t SET a = 1 WHERE b = ?lcName", "DELETE FROM t WHERE b = ?lcName", "x = 1 + ?y"] {
        assert!(!foxvm::parser::parse_program(src).diagnostics.is_empty(), "{src}");
    }
}

#[test]
fn insert_select_and_union_parse() {
    assert_eq!(
        common::ok("INSERT INTO t (a, b) SELECT x, y FROM s WHERE x > 1"),
        "(insert T (A B) (select (X Y) (from s:S) (where (> X 1))))"
    );
    assert_eq!(
        common::ok("SELECT a FROM s UNION ALL SELECT b FROM t ORDER BY 1 INTO CURSOR q"),
        "(select (A) (from s:S) (union-all (select (B) (from t:T))) (order 1) (into-cursor Q))"
    );
    assert_eq!(
        common::ok("SELECT a FROM s UNION SELECT b FROM t UNION ALL SELECT c FROM u INTO ARRAY laAll"),
        "(select (A) (from s:S) (union (select (B) (from t:T) (union-all (select (C) (from u:U))))) (into-array LAALL))"
    );
}

#[test]
fn quoted_names_after_do_and_create_table() {
    assert_eq!(common::ok("DO \"C:\\tools\\run.prg\" WITH 1"), "(do-expr \"C:\\\\tools\\\\run.prg\" (with 1))");
    let created = common::ok("CREATE TABLE 'APPREG01.DBF' NAME 'APPREG01' (KEYNAME C(70) NOT NULL)");
    assert!(created.starts_with("(create-table \"APPREG01.DBF\" ("), "{created}");
}

// ----- running -------------------------------------------------------------------------------

#[test]
fn use_in_select_closes_the_table_and_ignores_one_that_is_not_open() {
    let out = run("USE parts
? USED('parts')
USE IN SELECT('parts')
? USED('parts')
USE IN SELECT('parts')
? 'still here'");
    assert_eq!(out, [".T.", ".F.", "still here"]);
}

#[test]
fn use_in_select_leaves_the_selected_area_alone() {
    let out = run("USE parts
CREATE CURSOR other (a C(1))
USE IN SELECT('parts')
? ALIAS()");
    assert_eq!(out, ["OTHER"]);
}

#[test]
fn a_parameter_is_read_as_the_expression_it_marks() {
    // measured: the mark changes nothing about how the name is read. (In the product a name
    // that is both a field of a source and a variable is the field; this runtime lets a LOCAL
    // shadow a field, which is a gap of its own and not exercised here.)
    let out = run("LOCAL lcCode
lcCode = 'BBB'
SELECT qty FROM parts WHERE parts.code = ?lcCode INTO CURSOR q
? TRANSFORM(RECCOUNT()), TRANSFORM(qty)
SELECT qty FROM parts WHERE parts.code = ?m.lcCode INTO CURSOR q
? TRANSFORM(RECCOUNT()), TRANSFORM(qty)");
    assert_eq!(out, ["1 2", "1 2"]);
}

#[test]
fn insert_select_copies_rows_to_the_fields_named_in_order() {
    // the table's fields are the other way round from the select list
    let out = run("CREATE CURSOR bigger (qty N(3), code C(3))
INSERT INTO bigger (code, qty) SELECT code, qty FROM parts WHERE qty > 1
? TRANSFORM(RECCOUNT('bigger'))
SCAN
  ? code, TRANSFORM(qty)
ENDSCAN
? ALIAS(), USED('__ins1')");
    assert_eq!(out, ["2", "BBB 2", "CCC 3", "BIGGER .F."]);
}

#[test]
fn insert_select_without_a_field_list_takes_the_table_in_order() {
    let out = run("USE parts
CREATE CURSOR copy (code C(3), qty N(3))
SELECT parts
INSERT INTO copy SELECT * FROM parts
? TRANSFORM(RECCOUNT('copy')), ALIAS()
SELECT copy
GO 3
? code, TRANSFORM(qty)");
    assert_eq!(out, ["3 PARTS", "CCC 3"]);
}

#[test]
fn insert_select_into_a_table_named_by_an_expression() {
    let out = run("CREATE CURSOR copy (code C(3), qty N(3))
lcTarget = 'copy'
INSERT INTO (lcTarget) SELECT code, qty FROM parts WHERE qty = 2
? TRANSFORM(RECCOUNT('copy')), copy.code");
    assert_eq!(out, ["1 BBB"]);
}

#[test]
fn union_all_stacks_the_rows_under_the_first_select() {
    let out = run("SELECT 1 AS norder, 'top ' AS label, code FROM parts WHERE qty = 3 ;
  UNION ALL ;
  SELECT 2 AS norder, 'rest' AS label, code FROM parts WHERE qty < 3 ;
  ORDER BY norder, code INTO CURSOR q
? TRANSFORM(RECCOUNT('q')), ALIAS()
SCAN
  ? TRANSFORM(norder), label, code
ENDSCAN
? USED('__uni1'), USED('__uni1_1')");
    assert_eq!(out, ["3 Q", "1 top  CCC", "2 rest AAA", "2 rest BBB", ".F. .F."]);
}

#[test]
fn union_without_all_folds_rows_the_same_on_both_sides() {
    let out = run("SELECT code FROM parts UNION SELECT code FROM parts WHERE qty > 1 INTO CURSOR q
? TRANSFORM(RECCOUNT('q'))
SELECT code FROM parts UNION ALL SELECT code FROM parts WHERE qty > 1 INTO CURSOR q
? TRANSFORM(RECCOUNT('q'))");
    assert_eq!(out, ["3", "5"]);
}

#[test]
fn union_into_array_leaves_the_area_where_it_was() {
    let out = run("USE parts
SELECT code FROM parts WHERE qty = 1 UNION ALL SELECT code FROM parts WHERE qty = 3 INTO ARRAY laCodes
? TRANSFORM(ALEN(laCodes)), laCodes(1), laCodes(2), ALIAS()");
    assert_eq!(out, ["2 AAA CCC PARTS"]);
}

#[test]
fn a_query_into_an_array_puts_the_program_back_where_it_stood() {
    // the source is open in another area; opening it for the query must not move the program
    let out = run("USE parts
CREATE CURSOR other (a C(1))
SELECT code FROM parts WHERE qty = 1 INTO ARRAY laCodes
? ALIAS()");
    assert_eq!(out, ["OTHER"]);
}

#[test]
fn a_quoted_program_name_is_run() {
    let out = run("DO \"helper\" WITH 'x'
PROCEDURE helper
  LPARAMETERS c
  ? 'ran', c
ENDPROC");
    assert_eq!(out, ["ran x"]);
}
