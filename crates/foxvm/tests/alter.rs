//! ALTER TABLE and the locks: the shape of a table, and who holds it.

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

fn run(src: &str) -> Vec<String> {
    let mut host = host();
    match run_program(src, &mut host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

#[test]
fn alter_table_adds_a_column_and_keeps_the_records() {
    let out = run(
        r#"
USE customer.dbf
ALTER TABLE customer.dbf ADD COLUMN city C(12)
? FCOUNT(), FIELD(3), RECCOUNT()
GO TOP
? ALLTRIM(name), "[" + city + "]"
REPLACE city WITH "Bristol"
? ALLTRIM(city)
"#,
    );
    assert_eq!(out, vec!["         3 CITY          2", "Acme [            ]", "Bristol"]);
}

#[test]
fn alter_table_drops_and_renames_columns() {
    let out = run(
        r#"
USE customer.dbf
ALTER TABLE customer.dbf RENAME COLUMN name TO company
? FIELD(1), FCOUNT()
GO TOP
? ALLTRIM(company)
ALTER TABLE customer.dbf DROP COLUMN amount
? FCOUNT(), FIELD(1)
GO BOTTOM
? ALLTRIM(company)
"#,
    );
    assert_eq!(out, vec!["COMPANY          2", "Acme", "         1 COMPANY", "Beta"]);
}

#[test]
fn alter_table_widens_a_column() {
    let out = run(
        r#"
USE customer.dbf
ALTER TABLE customer.dbf ALTER COLUMN name C(20)
GO TOP
? LEN(name), ALLTRIM(name)
"#,
    );
    assert_eq!(out, vec!["        20 Acme"]);
}

#[test]
fn a_lock_is_granted_and_let_go_of() {
    let out = run(
        r#"
USE customer.dbf
? ISFLOCKED(), ISRLOCKED()
? RLOCK(), ISRLOCKED()
UNLOCK
? ISRLOCKED()
? FLOCK(), ISFLOCKED(), ISRLOCKED(2)
UNLOCK ALL
? ISFLOCKED()
? LOCK("2"), ISRLOCKED(2), ISRLOCKED(1)
"#,
    );
    assert_eq!(
        out,
        vec![".F. .F.", ".T. .T.", ".F.", ".T. .T. .T.", ".F.", ".T. .T. .F."]
    );
}
