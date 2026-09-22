//! PACK: the records marked deleted go, and the index follows the ones that move.

mod dbf_fixture;

use dbf_fixture::table_with_deleted;
use foxvm::mock_host::{MockHost, run_program};

fn host() -> MockHost {
    let bytes = table_with_deleted(
        &[("NAME", b'C', 10, 0), ("AMOUNT", b'N', 8, 2)],
        &[&["Acme", "100.00"], &["Beta", "200.00"], &["Cirrus", "300.00"], &["Delta", "400.00"]],
        &[2],
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
fn pack_takes_the_deleted_records_out() {
    let out = run(
        r#"
USE customer.dbf
? RECCOUNT()
PACK
? RECCOUNT(), RECNO()
GO TOP
? ALLTRIM(name)
SKIP
? ALLTRIM(name), RECNO()
GO BOTTOM
? ALLTRIM(name), RECNO()
"#,
    );
    assert_eq!(out, vec!["         4", "         3          1", "Acme", "Cirrus          2", "Delta          3"]);
}

#[test]
fn a_record_deleted_now_goes_when_the_table_is_packed() {
    let out = run(
        r#"
USE customer.dbf
GO 1
DELETE
PACK
? RECCOUNT()
GO TOP
? ALLTRIM(name)
"#,
    );
    assert_eq!(out, vec!["         2", "Cirrus"]);
}

#[test]
fn the_index_follows_the_records_that_moved() {
    let out = run(
        r#"
USE customer.dbf
INDEX ON name TAG byname
PACK
? RECCOUNT()
GO TOP
? ALLTRIM(name), RECNO()
SEEK "Delta"
? FOUND(), ALLTRIM(name), RECNO()
SEEK "Beta"
? FOUND()
"#,
    );
    // Beta was record 2 and was deleted, so it is not in the table or in the tag any more
    assert_eq!(out, vec!["         3", "Acme          1", ".T. Delta          3", ".F."]);
}
