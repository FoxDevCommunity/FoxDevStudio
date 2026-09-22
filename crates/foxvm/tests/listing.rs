//! What `LIST` and `DISPLAY` write, laid out the way Visual FoxPro lays it out.
//!
//! Every expectation here was read off Visual FoxPro 9 running the same commands over the same
//! table. The rules the product turned out to be following:
//!
//! - a row of field names goes above the records, and `SET HEADINGS OFF` leaves it out;
//! - it is written above the first record there is to write, so a listing that matches nothing
//!   writes nothing at all - not even the names;
//! - a column is as wide as the value it shows, or as wide as the field's name when the name is
//!   longer, with one space between columns;
//! - numbers line up on the right and so do their names, everything else on the left;
//! - `OFF` drops the record-number column and leaves the two spaces that followed it;
//! - `LIST` with nothing said about which records walks the table, `DISPLAY` shows the one the
//!   record pointer is on.

mod dbf_fixture;

use dbf_fixture::table;
use foxvm::mock_host::{MockHost, run_program};

fn host() -> MockHost {
    let bytes = table(
        &[("CODE", b'C', 4, 0), ("NAME", b'C', 10, 0), ("PRICE", b'N', 8, 2), ("OK", b'L', 1, 0)],
        &[&["A1", "Bolt", "2.50", "T"], &["B2", "Nut", "1.75", "F"], &["C3", "Washer", "0.40", "T"]],
    );
    let mut host = MockHost::new();
    host.tables.insert("PARTS.DBF".into(), bytes);
    host
}

fn run(src: &str) -> Vec<String> {
    match run_program(src, &mut host()) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

#[test]
fn list_writes_the_field_names_above_the_records() {
    let out = run("USE parts.dbf\nLIST\n");
    assert_eq!(
        out,
        vec![
            "Record#  CODE NAME          PRICE OK",
            "      1  A1   Bolt           2.50 .T.",
            "      2  B2   Nut            1.75 .F.",
            "      3  C3   Washer         0.40 .T.",
        ]
    );
}

#[test]
fn a_named_field_list_narrows_the_columns_to_those_fields() {
    let out = run("USE parts.dbf\nLIST code, price FOR price > 1\n");
    assert_eq!(out, vec!["Record#  CODE    PRICE", "      1  A1       2.50", "      2  B2       1.75"]);
}

#[test]
fn set_headings_off_leaves_the_row_of_names_out() {
    let out = run("USE parts.dbf\nSET HEADINGS OFF\nLIST code\n? SET(\"HEADINGS\")\n");
    assert_eq!(out, vec!["      1  A1", "      2  B2", "      3  C3", "OFF"]);
}

#[test]
fn off_drops_the_record_number() {
    let out = run("USE parts.dbf\nLIST code OFF\n");
    assert_eq!(out, vec!["  CODE", "  A1", "  B2", "  C3"]);
}

#[test]
fn a_listing_that_matches_nothing_writes_nothing_at_all() {
    let out = run("USE parts.dbf\nLIST FOR price > 99\n? \"after\"\n");
    assert_eq!(out, vec!["after"]);
}

#[test]
fn display_shows_the_record_the_pointer_is_on_and_list_shows_them_all() {
    let out = run("USE parts.dbf\nGO 2\nDISPLAY code\nDISPLAY ALL code\n");
    assert_eq!(
        out,
        vec![
            "Record#  CODE",
            "      2  B2",
            "Record#  CODE",
            "      1  A1",
            "      2  B2",
            "      3  C3",
        ]
    );
}

#[test]
fn a_column_is_as_wide_as_the_longer_of_the_name_and_the_value() {
    let out = run("CREATE CURSOR two (verylongname L, x C(20))\nINSERT INTO two VALUES (.T., \"hello\")\nLIST\n");
    assert_eq!(out, vec!["Record#  VERYLONGNAME X", "      1  .T.          hello"]);
}
