//! Buffering and transactions: what a table holds back, and what sends it on.

mod dbf_fixture;

use dbf_fixture::table;
use foxvm::mock_host::{MockHost, run_program};

fn host() -> MockHost {
    let bytes = table(
        &[("NAME", b'C', 10, 0), ("AMOUNT", b'N', 8, 2)],
        &[&["Acme", "100.00"], &["Beta", "200.00"]],
    );
    let mut host = MockHost::new();
    host.tables.insert("CUSTOMER.DBF".into(), bytes);
    host
}

fn run_on(host: &mut MockHost, src: &str) -> Vec<String> {
    match run_program(src, host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

fn run(src: &str) -> Vec<String> {
    run_on(&mut host(), src)
}

#[test]
fn a_buffered_change_waits_for_tableupdate() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
? CURSORGETPROP("Buffering")
? CURSORSETPROP("Buffering", 5), CURSORGETPROP("Buffering")
GO TOP
REPLACE name WITH "Acme Ltd"
* the program sees the change, and the file does not have it yet
? ALLTRIM(name), GETFLDSTATE(1), GETFLDSTATE(2)
? TABLEUPDATE(.T.)
? GETFLDSTATE(1)
USE customer.dbf
GO TOP
? ALLTRIM(name)
"#,
    );
    assert_eq!(out, vec!["         1", ".T.          5", "Acme Ltd          2          1", ".T.", "         1", "Acme Ltd"]);
}

#[test]
fn tablerevert_puts_the_record_back() {
    let out = run(
        r#"
USE customer.dbf
? CURSORSETPROP("Buffering", 5)
GO TOP
REPLACE name WITH "Not this"
? ALLTRIM(name)
? TABLEREVERT(.T.)
? ALLTRIM(name), GETFLDSTATE(1)
"#,
    );
    assert_eq!(out, vec![".T.", "Not this", "         1", "Acme          1"]);
}

#[test]
fn every_changed_record_is_held_and_found_again() {
    let out = run(
        r#"
USE customer.dbf
? CURSORSETPROP("Buffering", 5)
GO TOP
REPLACE amount WITH 111
GO BOTTOM
REPLACE amount WITH 222
? GETNEXTMODIFIED(0), GETNEXTMODIFIED(1), GETNEXTMODIFIED(2)
GO TOP
? amount
GO BOTTOM
? amount
? TABLEUPDATE(.T.)
? GETNEXTMODIFIED(0)
"#,
    );
    assert_eq!(out, vec![".T.", "         1          2          0", "  111.00", "  222.00", ".T.", "         0"]);
}

#[test]
fn a_transaction_holds_every_write_until_it_ends() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
? TXNLEVEL()
BEGIN TRANSACTION
? TXNLEVEL()
GO TOP
REPLACE name WITH "In flight"
END TRANSACTION
? TXNLEVEL()
USE customer.dbf
GO TOP
? ALLTRIM(name)
"#,
    );
    assert_eq!(out, vec!["         0", "         1", "         0", "In flight"]);
}

#[test]
fn a_rolled_back_transaction_changes_nothing() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
BEGIN TRANSACTION
GO TOP
REPLACE name WITH "Never"
ROLLBACK
? TXNLEVEL()
? ALLTRIM(name)
USE customer.dbf
GO TOP
? ALLTRIM(name)
"#,
    );
    assert_eq!(out, vec!["         0", "Acme", "Acme"]);
}

#[test]
fn a_record_appended_while_buffered_goes_when_it_is_reverted() {
    let out = run(
        r#"
USE customer.dbf
? CURSORSETPROP("Buffering", 5)
APPEND BLANK
REPLACE name WITH "Cirrus"
? RECCOUNT(), GETFLDSTATE(1)
? TABLEREVERT(.T.)
? RECCOUNT()
APPEND BLANK
REPLACE name WITH "Cirrus"
? TABLEUPDATE(.T.), RECCOUNT()
USE customer.dbf
GO BOTTOM
? ALLTRIM(name), RECCOUNT()
"#,
    );
    assert_eq!(out, vec![".T.", "         3          4", "         1", "         2", ".T.          3", "Cirrus          3"]);
}
