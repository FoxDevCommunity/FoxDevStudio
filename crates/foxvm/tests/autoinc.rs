//! Fields the table fills in itself: `AUTOINC NEXTVALUE n STEP s`.
//!
//! Visual FoxPro keeps what the field takes next in the table's own header, so the count
//! survives the table being closed and opened again, and every record gets the next one
//! whether it was added by INSERT or by APPEND BLANK.

use foxvm::mock_host::{MockHost, run_program};

fn run(host: &mut MockHost, src: &str) -> Vec<String> {
    match run_program(src, host) {
        Ok((_, output)) => output,
        Err(e) => panic!("{} at line {}: {}", e.code, e.line, e.message),
    }
}

const MAKE: &str = "CREATE TABLE ai FREE (iID i AUTOINC NEXTVALUE 1 STEP 1, name c(10))\n";

#[test]
fn every_record_added_gets_the_next_number() {
    let mut host = MockHost::new();
    let out = run(
        &mut host,
        &format!(
            "{MAKE}INSERT INTO ai (name) VALUES (\"Jane\")
INSERT INTO ai (name) VALUES (\"John\")
INSERT INTO ai (name) VALUES (\"Greg\")
GO TOP
SCAN
? iID, ALLTRIM(name)
ENDSCAN
"
        ),
    );
    assert_eq!(out, vec!["          1 Jane", "          2 John", "          3 Greg"]);
}

#[test]
fn the_starting_value_and_the_step_are_the_ones_the_table_was_given() {
    let mut host = MockHost::new();
    let out = run(
        &mut host,
        "CREATE TABLE ai FREE (iID i AUTOINC NEXTVALUE 100 STEP 5, name c(10))
INSERT INTO ai (name) VALUES (\"a\")
INSERT INTO ai (name) VALUES (\"b\")
GO TOP
? iID
SKIP
? iID
",
    );
    assert_eq!(out, vec!["        100", "        105"]);
}

#[test]
fn append_blank_counts_the_same_way_an_insert_does() {
    let mut host = MockHost::new();
    let out = run(
        &mut host,
        &format!(
            "{MAKE}APPEND BLANK
REPLACE name WITH \"first\"
APPEND BLANK
REPLACE name WITH \"second\"
GO TOP
? iID
GO BOTTOM
? iID
"
        ),
    );
    assert_eq!(out, vec!["          1", "          2"]);
}

#[test]
fn what_the_field_takes_next_is_kept_in_the_table() {
    let mut host = MockHost::new();
    let out = run(
        &mut host,
        &format!(
            "{MAKE}INSERT INTO ai (name) VALUES (\"a\")
INSERT INTO ai (name) VALUES (\"b\")
? GETAUTOINCVALUE()
USE
USE ai
? GETAUTOINCVALUE()
INSERT INTO ai (name) VALUES (\"c\")
GO BOTTOM
? iID
"
        ),
    );
    // GETAUTOINCVALUE() answers the last value taken (2, from "b"), not the next one the table
    // would give out (3) - measured against vfp9.exe, and eleven wide there, the field's own
    // Integer width. The count itself carries on where it left off after the table is closed
    // and opened again, which is what the third row still shows.
    assert_eq!(out, vec!["          2", "          2", "          3"]);
}

#[test]
fn a_table_with_no_such_field_has_no_such_number() {
    let mut host = MockHost::new();
    let out = run(
        &mut host,
        "CREATE TABLE plain FREE (name c(10))
INSERT INTO plain VALUES (\"a\")
? GETAUTOINCVALUE()
",
    );
    // measured against vfp9.exe: a table with nothing autoincrementing has generated no value
    // at all, which GETAUTOINCVALUE() answers as .NULL. rather than 0
    assert_eq!(out, vec![".NULL."]);
}
