//! COPY TO, COPY STRUCTURE, SORT TO and TOTAL ON: a new table made from an open one.

mod dbf_fixture;

use dbf_fixture::table;
use foxvm::mock_host::{MockHost, run_program};

fn host() -> MockHost {
    let bytes = table(
        &[("CUSTNO", b'C', 6, 0), ("NAME", b'C', 10, 0), ("AMOUNT", b'N', 8, 2)],
        &[
            &["A100", "Acme", "125.50"],
            &["B200", "Beta", "80.00"],
            &["A100", "Acme", "200.00"],
            &["C300", "Cirrus", "50.25"],
        ],
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

#[test]
fn copy_to_writes_a_table_that_reads_back() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
COPY TO backup.dbf
USE backup.dbf
? RECCOUNT(), FCOUNT()
? ALLTRIM(name), amount
GO BOTTOM
? ALLTRIM(name)
"#,
    );
    assert_eq!(out, vec!["         4          3", "Acme   125.50", "Cirrus"]);
    assert!(host.tables.contains_key("BACKUP.DBF"));
}

#[test]
fn a_copy_can_take_some_of_the_records_and_some_of_the_columns() {
    let out = run_on(
        &mut host(),
        r#"
USE customer.dbf
COPY TO big.dbf FIELDS name, amount FOR amount > 100
USE big.dbf
? RECCOUNT(), FCOUNT(), FIELD(1), FIELD(2)
? ALLTRIM(name)
SKIP
? ALLTRIM(name)
"#,
    );
    assert_eq!(out, vec!["         2          2 NAME AMOUNT", "Acme", "Acme"]);
}

#[test]
fn copy_structure_writes_the_columns_and_no_records() {
    let out = run_on(
        &mut host(),
        r#"
USE customer.dbf
COPY STRUCTURE TO empty.dbf
USE empty.dbf
? RECCOUNT(), FCOUNT(), EOF()
"#,
    );
    assert_eq!(out, vec!["         0          3 .T."]);
}

#[test]
fn sort_writes_the_records_in_the_order_of_its_keys() {
    let out = run_on(
        &mut host(),
        r#"
USE customer.dbf
SORT TO byamount.dbf ON amount
USE byamount.dbf
? ALLTRIM(name), amount
GO BOTTOM
? ALLTRIM(name), amount
USE
SELECT 1
USE customer.dbf
SORT TO down.dbf ON amount /D
USE down.dbf
? ALLTRIM(name), amount
"#,
    );
    assert_eq!(out, vec!["Cirrus    50.25", "Acme   200.00", "Acme   200.00"]);
}

#[test]
fn total_adds_up_a_run_of_records_with_the_same_key() {
    let out = run_on(
        &mut host(),
        r#"
USE customer.dbf
SORT TO sorted.dbf ON custno
USE sorted.dbf
TOTAL ON custno TO totals.dbf
USE totals.dbf
? RECCOUNT()
? ALLTRIM(custno), amount
SKIP
? ALLTRIM(custno), amount
"#,
    );
    assert_eq!(out, vec!["         3", "A100   325.50", "B200    80.00"]);
}

#[test]
fn a_copy_can_be_text_instead_of_a_table() {
    let mut host = host();
    run_on(
        &mut host,
        r#"
USE customer.dbf
COPY TO rows.csv TYPE CSV
COPY TO rows.txt TYPE SDF
"#,
    );
    let csv = String::from_utf8(host.files["ROWS.CSV"].clone()).unwrap();
    assert_eq!(csv.lines().next(), Some("\"A100\",\"Acme\",125.50"));
    let sdf = String::from_utf8(host.files["ROWS.TXT"].clone()).unwrap();
    assert_eq!(sdf.lines().next(), Some("A100  Acme        125.50"));
}

#[test]
fn drop_table_takes_the_file_away() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
COPY TO spare.dbf
USE spare.dbf
? RECCOUNT()
USE
DROP TABLE spare.dbf
"#,
    );
    assert_eq!(out, vec!["         4"]);
    assert!(!host.tables.contains_key("SPARE.DBF"), "the file is gone");
}

#[test]
fn append_from_adds_another_table_to_the_end() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
COPY TO more.dbf FOR amount > 100
USE customer.dbf
APPEND FROM more.dbf
? RECCOUNT()
GO BOTTOM
? ALLTRIM(name), amount
USE customer.dbf
? RECCOUNT()
"#,
    );
    assert_eq!(out, vec!["         6", "Acme   200.00", "         6"]);
}

#[test]
fn append_from_takes_a_condition_and_a_field_list() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
COPY TO source.dbf
USE customer.dbf
APPEND FROM source.dbf FOR amount > 100
? RECCOUNT()
GO BOTTOM
? ALLTRIM(name), amount
"#,
    );
    // the condition is tested against the record as it arrives
    assert_eq!(out, vec!["         6", "Acme   200.00"]);
}

#[test]
fn append_from_reads_a_text_file() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
COPY TO out.txt TYPE SDF
ZAP
? RECCOUNT()
APPEND FROM out.txt TYPE SDF
? RECCOUNT()
GO TOP
? ALLTRIM(custno), ALLTRIM(name), amount
"#,
    );
    assert_eq!(out, vec!["         0", "         4", "A100 Acme   125.50"]);
}

// ----- the structure as data, and a table made back from it ---------------------------------

#[test]
fn copy_structure_extended_writes_one_record_for_each_field() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
COPY STRUCTURE EXTENDED TO shape.dbf
USE shape.dbf
? RECCOUNT(), FCOUNT()
SCAN
  ? ALLTRIM(field_name), field_type, field_len, field_dec
ENDSCAN
"#,
    );
    // eighteen columns, as Visual FoxPro 9 writes them, and a record per field of the table
    assert_eq!(
        out,
        vec![
            "         3         18", "CUSTNO C   6   0", "NAME C  10   0", "AMOUNT N   8   2",
        ]
    );
}

#[test]
fn create_from_makes_the_table_the_description_describes() {
    let mut host = host();
    let out = run_on(
        &mut host,
        r#"
USE customer.dbf
COPY STRUCTURE EXTENDED TO shape.dbf
USE shape.dbf
REPLACE field_name WITH "TOTAL", field_type WITH "N", field_len WITH 12, field_dec WITH 4 FOR ALLTRIM(field_name) == "AMOUNT"
USE
CREATE backup.dbf FROM shape.dbf
? ALIAS(), RECCOUNT(), FCOUNT()
? FIELD(1), FIELD(3), TYPE("total")
"#,
    );
    // the new table is the one selected afterwards, empty, with the changed column in it
    assert_eq!(out, vec!["BACKUP          0          3", "CUSTNO TOTAL N"]);
    assert!(host.tables.contains_key("BACKUP.DBF"));
}

#[test]
fn create_from_says_so_when_what_it_reads_is_not_a_description() {
    let mut host = host();
    let err = run_program("USE customer.dbf\nUSE\nCREATE backup.dbf FROM customer.dbf", &mut host)
        .err()
        .expect("a table that is not a description");
    assert!(err.message.contains("does not \\n             describe a structure") || err.message.contains("describe a structure"), "{}", err.message);
}
