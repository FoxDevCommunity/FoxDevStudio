//! The database container: making one, opening it, and what it holds.

use foxvm::mock_host::{MockHost, run_program};

fn run_on(host: &mut MockHost, src: &str) -> Vec<String> {
    match run_program(src, host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

#[test]
fn a_database_is_made_and_opened_again() {
    let mut host = MockHost::new();
    run_on(
        &mut host,
        r#"
CREATE DATABASE sales
CREATE TABLE orders (code C(6), amount N(8,2))
USE
"#,
    );
    assert!(host.files.contains_key("SALES.DBC"), "the container was written");
    assert!(host.files.contains_key("SALES.DCT"), "so was the memo file beside it");

    // a fresh program opens it and finds the table listed
    let out = run_on(
        &mut host,
        r#"
OPEN DATABASE sales
LIST DATABASE
"#,
    );
    assert_eq!(out, vec!["Database   sales", "Table      ORDERS"]);
}

#[test]
fn a_table_can_be_taken_out_of_a_database_and_renamed_in_it() {
    let mut host = MockHost::new();
    let out = run_on(
        &mut host,
        r#"
CREATE DATABASE shop
CREATE TABLE goods (code C(6))
USE
RENAME TABLE goods TO stock
LIST DATABASE
REMOVE TABLE stock
LIST DATABASE
VALIDATE DATABASE
"#,
    );
    assert_eq!(
        out,
        vec![
            "Database   shop",
            "Table      STOCK",
            "Database   shop",
            // the four lines the product writes, measured
            "Validate Database SHOP:",
            "Rebuilding structural index....  Index rebuilt.",
            "Database container is valid.",
            "",
        ]
    );
}

#[test]
fn a_view_keeps_the_select_it_stands_for() {
    let mut host = MockHost::new();
    run_on(
        &mut host,
        r#"
CREATE DATABASE books
CREATE TABLE titles (name C(10), price N(6,2))
USE
CREATE SQL VIEW dear AS SELECT * FROM titles WHERE price > 20
"#,
    );
    let out = run_on(
        &mut host,
        r#"
OPEN DATABASE books
LIST DATABASE
"#,
    );
    assert_eq!(out, vec!["Database   books", "Table      TITLES", "View       DEAR"]);
}

#[test]
fn the_functions_report_on_the_database_that_is_current() {
    let mut host = MockHost::new();
    let out = run_on(
        &mut host,
        r#"
? DBC() == "", DBUSED("shop")
CREATE DATABASE shop
? DBC(), DBUSED("shop")
CREATE TABLE goods (code C(6))
USE
? INDBC("goods", "TABLE"), INDBC("nothing", "TABLE")
LOCAL aNames(1)
? ADBOBJECTS(aNames, "TABLE"), aNames(1)
? DBSETPROP("goods", "TABLE", "Comment", "what we sell")
? DBGETPROP("goods", "TABLE", "Comment")
? DBGETPROP("shop", "DATABASE", "Comment") == ""
CREATE SQL VIEW cheap AS SELECT * FROM goods
? DBGETPROP("cheap", "VIEW", "SQL")
SET DATABASE TO
? DBC() == ""
SET DATABASE TO shop
? DBUSED("shop"), DBC() == "shop.dbc"
CLOSE DATABASES
? DBC() == "", DBUSED("shop")
"#,
    );
    assert_eq!(
        out,
        vec![
            ".T. .F.", "shop.dbc .T.", ".T. .F.", "         1 GOODS", ".T.", "what we sell", ".T.", "SELECT * FROM goods", ".T.", ".T. .T.", ".T. .F.",
        ]
    );
}

#[test]
fn using_a_view_runs_the_select_it_stands_for() {
    let mut host = MockHost::new();
    let out = run_on(
        &mut host,
        r#"
CREATE DATABASE library
CREATE TABLE books (title C(12), price N(6,2))
INSERT INTO books (title, price) VALUES ("Cheap one", 5)
INSERT INTO books (title, price) VALUES ("Dear one", 50)
INSERT INTO books (title, price) VALUES ("Dearer one", 80)
USE
CREATE SQL VIEW dear AS SELECT title, price FROM books WHERE price > 20
USE dear
? ALIAS(), RECCOUNT()
GO TOP
? ALLTRIM(title)
SKIP
? ALLTRIM(title)
"#,
    );
    assert_eq!(out, vec!["DEAR          2", "Dear one", "Dearer one"]);
}

#[test]
fn the_stored_procedures_of_a_database_are_told_what_happens_to_it() {
    let mut host = MockHost::new();
    host.files.insert(
        "PROCS.PRG".into(),
        br#"PROCEDURE dbc_BeforeCreateTable
LPARAMETERS cFile, cTable
? "about to make " + ALLTRIM(cTable)
RETURN .T.

PROCEDURE dbc_AfterCreateTable
LPARAMETERS cFile, cTable
? "made " + ALLTRIM(cTable)

PROCEDURE dbc_BeforeRemoveTable
LPARAMETERS cTable, lDelete, lRecycle
? "will not remove " + ALLTRIM(cTable)
RETURN .F.
"#
        .to_vec(),
    );
    let out = run_on(
        &mut host,
        r#"
CREATE DATABASE works
APPEND PROCEDURES FROM procs.prg
* until DBCEvents is on the product calls none of these procedures at all - measured
= DBSETPROP("works", "Database", "DBCEvents", .T.)
CREATE TABLE jobs (code C(6))
USE
LIST DATABASE
REMOVE TABLE jobs
LIST DATABASE
"#,
    );
    assert_eq!(
        out,
        vec![
            // every name a dbc event is handed arrives lower-cased
            "about to make jobs",
            "made jobs",
            "Database   works",
            "Table      JOBS",
            // the Before procedure said no, so the table is still there
            "will not remove jobs",
            "Database   works",
            "Table      JOBS",
        ]
    );
}

#[test]
fn the_procedures_are_kept_in_the_container_and_come_back_with_it() {
    let mut host = MockHost::new();
    // the event is handed the container and the three clauses OPEN DATABASE may carry, so a
    // procedure that wants to hear it has to declare them
    host.files.insert(
        "PROCS.PRG".into(),
        b"PROCEDURE dbc_OpenData\nLPARAMETERS cDb, lExclusive, lNoUpdate, lValidate\n? \"opened\"\n".to_vec(),
    );
    run_on(
        &mut host,
        r#"
CREATE DATABASE store
APPEND PROCEDURES FROM procs.prg
= DBSETPROP("store", "Database", "DBCEvents", .T.)
COPY PROCEDURES TO copy.prg
"#,
    );
    assert!(String::from_utf8(host.files["COPY.PRG"].clone()).unwrap().contains("dbc_OpenData"));

    // opening it again reads the procedures back, and they run
    let out = run_on(&mut host, "OPEN DATABASE store\n");
    assert_eq!(out, vec!["opened"]);
}

#[test]
fn list_memory_writes_out_the_variables_a_program_made() {
    let mut host = MockHost::new();
    let out = run_on(
        &mut host,
        r#"
PUBLIC gcTitle, gnCount
gcTitle = "the parts list"
gnCount = 3
LIST MEMORY
"#,
    );
    assert!(out.iter().any(|l| l.starts_with("GCTITLE") && l.contains("the parts list")), "{out:?}");
    assert!(out.iter().any(|l| l.starts_with("GNCOUNT") && l.contains('3')), "{out:?}");
    // the system variables are memory variables too, as they are in Visual FoxPro
    assert!(out.iter().any(|l| l.starts_with("_WINDOWS")), "{out:?}");
    assert!(out.iter().any(|l| l.ends_with("variables defined")), "{out:?}");
}

/// `dbname!tablename`: a table reached through the container it belongs to.
///
/// Every line of this was measured against Visual FoxPro 9 on the containers it ships. The
/// container records a path of its own for each table, so what it says is what is opened, and
/// the name it holds the table under is the alias even when the file is called something else.
#[test]
fn a_table_is_reached_through_the_database_that_holds_it() {
    let mut host = MockHost::new();
    run_on(
        &mut host,
        r#"
CREATE DATABASE mydb
CREATE TABLE shortf (a C(5))
INSERT INTO shortf VALUES ("one")
USE
RENAME TABLE shortf TO aVeryLongTableName
CLOSE DATABASES ALL
"#,
    );

    // the container is not open, so it is opened to answer the name, and left open without
    // becoming the current one
    let out = run_on(
        &mut host,
        r#"
USE mydb!aVeryLongTableName IN 0
? USED("aVeryLongTableName"), ALIAS(SELECT("aVeryLongTableName"))
? UPPER(JUSTFNAME(DBF("aVeryLongTableName")))
? ADATABASES(aD), DBC() == ""
"#,
    );
    assert_eq!(out, vec![".T. AVERYLONGTABLENAME", "SHORTF.DBF", "         1 .T."]);

    // with the container current a bare name finds its table too
    let out = run_on(
        &mut host,
        r#"
OPEN DATABASE mydb
USE aVeryLongTableName IN 0
? UPPER(JUSTFNAME(DBF("aVeryLongTableName")))
"#,
    );
    assert_eq!(out.last().map(String::as_str), Some("SHORTF.DBF"));

    // a name the container does not hold is an ordinary file, and says so when it is not there
    let mut host2 = MockHost::new();
    run_on(&mut host2, "CREATE DATABASE mydb\nCREATE TABLE t1 (a C(5))\nUSE\nCLOSE DATABASES ALL\n");
    match run_program("USE mydb!nosuch IN 0\n", &mut host2) {
        Ok((_, out)) => panic!("expected an error, got {out:?}"),
        Err(e) => {
            let said = e.message.to_ascii_uppercase();
            assert!(said.contains("NOSUCH") && !said.contains("MYDB"), "{}", e.message);
        }
    }

    // and a container that is not there is reported as the container, not as the table
    match run_program("USE nosuchdb!t1 IN 0\n", &mut host2) {
        Ok((_, out)) => panic!("expected an error, got {out:?}"),
        Err(e) => assert!(e.message.to_ascii_uppercase().contains("NOSUCHDB.DBC"), "{}", e.message),
    }
}
