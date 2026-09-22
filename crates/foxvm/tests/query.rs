//! `SELECT-SQL` against real tables: the loops, the gathering, and what happens to the rows.

mod dbf_fixture;

use dbf_fixture::{table, table_with_deleted};
use foxvm::mock_host::{MockHost, run_program};

/// Five customers in three states, and the orders that belong to some of them.
fn host() -> MockHost {
    let customers = table_with_deleted(
        &[("CUSTNO", b'C', 6, 0), ("NAME", b'C', 10, 0), ("STATE", b'C', 2, 0)],
        &[
            &["A100", "Acme", "NY"],
            &["B200", "Beta", "CA"],
            &["C300", "Cirrus", "NY"],
            &["D400", "Delta", "TX"],
            &["E500", "Echo", "NY"],
        ],
        &[],
    );
    let orders = table(
        &[("CUSTNO", b'C', 6, 0), ("AMOUNT", b'N', 9, 2)],
        &[
            &["A100", "100.00"],
            &["A100", "250.00"],
            &["B200", "75.50"],
            &["C300", "1000.00"],
        ],
    );
    let mut host = MockHost::new();
    host.tables.insert("CUSTOMER.DBF".into(), customers);
    host.tables.insert("ORDERS.DBF".into(), orders);
    host
}

fn run(src: &str) -> Vec<String> {
    let mut host = host();
    match run_program(src, &mut host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

/// Walks a cursor, printing one line per row, so a query result is checked the way a program
/// would read it rather than through a back door.
const DUMP: &str = r#"
PROCEDURE Dump(cAlias, cExpr)
  SELECT (cAlias)
  GO TOP
  DO WHILE NOT EOF()
    ? EVALUATE(cExpr)
    SKIP
  ENDDO
"#;

#[test]
fn a_query_gathers_the_rows_that_match() {
    let out = run(&format!(
        r#"
SELECT custno, name FROM customer.dbf WHERE state = "NY" INTO CURSOR ny
? ALIAS()
? RECCOUNT()
DO Dump WITH "ny", "ALLTRIM(custno) + '/' + ALLTRIM(name)"
{DUMP}"#
    ));
    assert_eq!(out, vec!["NY", "         3", "A100/Acme", "C300/Cirrus", "E500/Echo"]);
}

#[test]
fn star_takes_every_field_of_the_source() {
    let out = run(
        r#"
SELECT * FROM customer.dbf WHERE state = "TX" INTO CURSOR one
? FCOUNT()
? FIELD(1) + " " + FIELD(2) + " " + FIELD(3)
? RECCOUNT()
? ALLTRIM(one.name)
"#,
    );
    assert_eq!(out, vec!["         3", "CUSTNO NAME STATE", "         1", "Delta"]);
}

#[test]
fn order_by_and_top_and_distinct() {
    let out = run(&format!(
        r#"
SELECT DISTINCT state FROM customer.dbf ORDER BY state INTO CURSOR s
? RECCOUNT()
DO Dump WITH "s", "state"
SELECT TOP 2 name FROM customer.dbf ORDER BY name DESC INTO CURSOR t
DO Dump WITH "t", "ALLTRIM(name)"
{DUMP}"#
    ));
    assert_eq!(out, vec!["         3", "CA", "NY", "TX", "Echo", "Delta"]);
}

#[test]
fn aggregates_fold_the_whole_table_or_one_group_at_a_time() {
    let out = run(&format!(
        r#"
SELECT COUNT(*) AS n, SUM(amount) AS total, MAX(amount) AS biggest FROM orders.dbf INTO CURSOR all
? all.n
? all.total
? all.biggest

SELECT custno, COUNT(*) AS orders, SUM(amount) AS total FROM orders.dbf GROUP BY custno INTO CURSOR g
? RECCOUNT()
DO Dump WITH "g", "ALLTRIM(custno) + ':' + LTRIM(STR(orders)) + ':' + LTRIM(STR(total, 10, 2))"
{DUMP}"#
    ));
    // a whole number prints without decimals: values are f64 here, so a column has no scale
    assert_eq!(out, vec!["            4.000000", "         1425.500000", "         1000.000000", "         3", "A100:2:350.00", "B200:1:75.50", "C300:1:1000.00"]);
}

#[test]
fn two_sources_join_on_a_condition() {
    let out = run(&format!(
        r#"
SELECT customer.name, orders.amount FROM customer.dbf, orders.dbf ;
  WHERE customer.custno = orders.custno ORDER BY orders.amount INTO CURSOR j
? RECCOUNT()
DO Dump WITH "j", "ALLTRIM(name) + '=' + LTRIM(STR(amount, 10, 2))"
{DUMP}"#
    ));
    assert_eq!(out, vec!["         4", "Beta=75.50", "Acme=100.00", "Acme=250.00", "Cirrus=1000.00"]);
}

#[test]
fn a_left_join_keeps_the_rows_that_matched_nothing() {
    let out = run(&format!(
        r#"
SELECT customer.name, orders.amount FROM customer.dbf ;
  LEFT JOIN orders.dbf ON customer.custno = orders.custno INTO CURSOR j
? RECCOUNT()
DO Dump WITH "j", "ALLTRIM(name) + '=' + LTRIM(STR(amount, 10, 2))"
{DUMP}"#
    ));
    // the side that matched nothing reads as .NULL., which is what Visual FoxPro puts there,
    // so the whole expression each of those two rows is built from comes out .NULL. as well
    assert_eq!(
        out,
        vec!["         6", "Acme=100.00", "Acme=250.00", "Beta=75.50", "Cirrus=1000.00", ".NULL.", ".NULL."]
    );
}

#[test]
fn into_array_gives_a_two_dimensional_array() {
    let out = run(
        r#"
SELECT custno, state FROM customer.dbf WHERE state = "NY" INTO ARRAY aRows
? ALEN(aRows, 1)
? ALEN(aRows, 2)
? ALLTRIM(aRows[1, 1])
? aRows[3, 2]
"#,
    );
    assert_eq!(out, vec!["         3", "         2", "A100", "NY"]);
}

#[test]
fn a_query_leaves_the_table_it_borrowed_where_it_found_it() {
    let out = run(
        r#"
USE customer.dbf
GO 4
SELECT custno FROM customer ORDER BY custno INTO CURSOR q
? RECNO("customer")
? ALLTRIM(customer.custno)
? ALIAS()
? RECCOUNT("q")
"#,
    );
    assert_eq!(out, vec!["         4", "D400", "Q", "         5"]);
}

#[test]
fn a_query_over_a_query_result_reads_it_like_a_table() {
    let out = run(
        r#"
SELECT custno, state FROM customer.dbf INTO CURSOR first
SELECT COUNT(*) AS n FROM first WHERE state = "NY" INTO CURSOR second
? second.n
"#,
    );
    assert_eq!(out, vec!["            3.000000"]);
}

// ---- subqueries --------------------------------------------------------------------------------

#[test]
fn a_subquery_answers_which_rows_belong() {
    let out = run(&format!(
        r#"
SELECT name FROM customer.dbf ;
  WHERE custno IN (SELECT custno FROM orders.dbf) ORDER BY name INTO CURSOR bought
? RECCOUNT()
DO Dump WITH "bought", "ALLTRIM(name)"
{DUMP}"#
    ));
    assert_eq!(out, vec!["         3", "Acme", "Beta", "Cirrus"]);
}

#[test]
fn some_and_any_ask_the_same_question_as_in() {
    // the shape the Visual FoxPro samples use: a comparison against SOME(subquery)
    let out = run(&format!(
        r#"
SELECT name FROM customer.dbf ;
  WHERE custno = SOME(SELECT custno FROM orders.dbf WHERE amount > 200) ;
  ORDER BY name INTO CURSOR big
? RECCOUNT()
DO Dump WITH "big", "ALLTRIM(name)"
{DUMP}"#
    ));
    assert_eq!(out, vec!["         2", "Acme", "Cirrus"]);
}

#[test]
fn not_in_keeps_the_rows_the_subquery_left_out() {
    let out = run(&format!(
        r#"
SELECT name FROM customer.dbf ;
  WHERE custno NOT IN (SELECT custno FROM orders.dbf) ORDER BY name INTO CURSOR quiet
? RECCOUNT()
DO Dump WITH "quiet", "ALLTRIM(name)"
{DUMP}"#
    ));
    assert_eq!(out, vec!["         2", "Delta", "Echo"]);
}

#[test]
fn exists_asks_only_whether_the_subquery_found_anything() {
    let out = run(
        r#"
SELECT name FROM customer.dbf WHERE EXISTS(SELECT custno FROM orders.dbf WHERE amount > 900) ;
  INTO CURSOR any_big
? RECCOUNT()
SELECT name FROM customer.dbf WHERE EXISTS(SELECT custno FROM orders.dbf WHERE amount > 9000) ;
  INTO CURSOR none_big
? RECCOUNT()
"#,
    );
    assert_eq!(out, vec!["         5", "         0"]);
}

#[test]
fn a_query_lets_go_of_the_cursor_its_subquery_used() {
    let out = run(
        r#"
SELECT name FROM customer.dbf WHERE custno IN (SELECT custno FROM orders.dbf) INTO CURSOR r
? RECCOUNT()
? USED("__SUB1")
? ALIAS()
"#,
    );
    assert_eq!(out, vec!["         3", ".F.", "R"]);
}

#[test]
fn into_array_can_name_a_property_rather_than_a_variable() {
    let out = run(
        r#"
PUBLIC oHolder
oHolder = CREATEOBJECT("Empty")
ADDPROPERTY(oHolder, "aRows", .F.)
SELECT custno FROM customer.dbf WHERE state = "NY" INTO ARRAY oHolder.aRows
? ALEN(oHolder.aRows)
? ALLTRIM(oHolder.aRows[1])
"#,
    );
    assert_eq!(out, vec!["         3", "A100"]);
}
